use std::alloc::{alloc, Layout};
use std::fmt::{Debug, Display, Formatter};
use std::intrinsics::likely;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use std::thread::Scope;

use compact_str::{format_compact, CompactString, ToCompactString};
use zune_inflate::{DeflateDecoder, DeflateOptions};

use crate::assets::{BASE_TEXT_Z, BASE_Z, CHUNK_UV, CONNECTION_UV, HEADER_SIZE, LINE_NUMBER_CONNECTOR_Z, LINE_NUMBER_SEPARATOR_UV, REGION_UV};
use crate::elements::compound::NbtCompound;
use crate::elements::element::NbtElement;
use crate::elements::list::{ValueIterator, ValueMutIterator};
use crate::encoder::UncheckedBufWriter;
use crate::tab::FileFormat;
use crate::vertex_buffer_builder::VertexBufferBuilder;
use crate::{DropFn, RenderContext, StrExt};

#[repr(C)]
pub struct NbtRegion {
	pub chunks: Box<(Vec<u16>, [NbtElement; 32 * 32])>,
	height: u32,
	true_height: u32,
	max_depth: u32,
	open: bool,
}

impl Clone for NbtRegion {
	#[allow(clippy::cast_ptr_alignment)]
	#[inline]
	fn clone(&self) -> Self {
		unsafe {
			let (map, chunks) = &*self.chunks;
			let boxx = alloc(Layout::new::<(Vec<u16>, [NbtElement; 32 * 32])>()).cast::<(Vec<u16>, [NbtElement; 32 * 32])>();
			let mapp = alloc(Layout::array::<u16>(map.len()).unwrap_unchecked()).cast::<u16>();
			let chunkss = alloc(Layout::array::<NbtElement>(32 * 32).unwrap_unchecked()).cast::<NbtElement>();
			map.as_ptr().copy_to_nonoverlapping(mapp, map.len());
			for n in 0..1024 {
				chunkss.add(n).write(chunks.get_unchecked(n).clone());
			}
			boxx.write((Vec::from_raw_parts(mapp, map.len(), map.len()), chunkss.cast::<[NbtElement; 32 * 32]>().read()));

			Self {
				chunks: Box::from_raw(boxx),
				height: self.height,
				true_height: self.true_height,
				max_depth: self.max_depth,
				open: self.open,
			}
		}
	}
}

impl Default for NbtRegion {
	fn default() -> Self {
		Self {
			chunks: Box::new((Vec::new(), unsafe { core::mem::zeroed() })),
			height: 1,
			true_height: 1,
			open: false,
			max_depth: 0,
		}
	}
}

impl NbtRegion {
	pub const ID: u8 = 128;
	pub const CHUNK_BANDWIDTH: usize = 32;

	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	#[must_use]
	pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
		fn parse(offset: u32, bytes: &[u8]) -> Option<(FileFormat, NbtElement)> {
			if offset < 512 {
				return Some((FileFormat::Zlib, unsafe { core::mem::zeroed() }));
			}

			let len = (offset as usize & 0xFF) * 4096;
			let offset = ((offset >> 8) - 2) as usize * 4096;
			if bytes.len() < offset + len {
				return None;
			}
			let data = &bytes[offset..(offset + len)];

			if let &[a, b, c, d, compression, ref data @ ..] = data {
				let chunk_len = (u32::from_be_bytes([a, b, c, d]) as usize).checked_sub(1)?;
				if data.len() < chunk_len {
					return None;
				}
				let data = &data[..chunk_len];
				let (compression, element) = match compression {
					1 => (
						FileFormat::Gzip,
						NbtElement::from_file(&DeflateDecoder::new_with_options(data, DeflateOptions::default().set_confirm_checksum(false)).decode_gzip().ok()?)?,
					),
					2 => (
						FileFormat::Zlib,
						NbtElement::from_file(&DeflateDecoder::new_with_options(data, DeflateOptions::default().set_confirm_checksum(false)).decode_zlib().ok()?)?,
					),
					3 => (FileFormat::Nbt, NbtElement::from_file(data)?),
					_ => return None,
				};
				if element.id() != NbtCompound::ID {
					return None;
				}
				Some((compression, element))
			} else {
				None
			}
		}

		if bytes.len() < 8192 {
			return None;
		}

		std::thread::scope(move |s| {
			let mut region = Self::new();

			let (&offsets, bytes) = bytes.split_array_ref::<4096>();
			let (&timestamps, bytes) = bytes.split_array_ref::<4096>();
			let mut threads = Vec::new();

			for (&offset, &timestamp) in offsets.array_chunks::<4>().zip(timestamps.array_chunks::<4>()) {
				let timestamp = u32::from_be_bytes(timestamp);
				let offset = u32::from_be_bytes(offset);
				threads.push((timestamp, s.spawn(move || parse(offset, bytes))));
			}

			unsafe {
				for (pos, (timestamp, thread)) in threads.into_iter().enumerate() {
					let (format, element) = thread.join().ok()??;
					region.insert_unchecked(
						pos,
						region.len(),
						NbtElement::Chunk(NbtChunk::from_compound(core::mem::transmute(element), ((pos >> 5) as u8 & 31, pos as u8 & 31), format, timestamp)),
					);
				}
			}

			Some(region)
		})
	}
	pub fn to_bytes(&self, writer: &mut UncheckedBufWriter) {
		unsafe {
			std::thread::scope(move |s| {
				let mut chunks = Vec::with_capacity(1024);
				for chunk in &self.chunks.as_ref().1 {
					chunks.push(s.spawn(move || {
						if chunk.is_null() {
							(vec![], 0)
						} else {
							let chunk = &(chunk as *const NbtElement).cast::<ManuallyDrop<NbtChunk>>().read();
							let mut writer = UncheckedBufWriter::new();
							chunk.to_bytes(&mut writer);
							(writer.finish(), chunk.last_modified)
						}
					}));
				}
				let mut o = 2_u32;
				let mut offsets = MaybeUninit::<u32>::uninit_array::<1024>();
				let mut timestamps = MaybeUninit::<u32>::uninit_array::<1024>();
				let mut new_chunks = Vec::with_capacity(chunks.len());
				for (chunk, (offset, timestamp)) in chunks.into_iter().zip(offsets.iter_mut().zip(timestamps.iter_mut())) {
					let Ok((chunk, last_modified)) = chunk.join() else {
						return;
					};
					let sectors = (chunk.len() / 4096) as u32;
					if sectors > 0 {
						offset.write((o.to_be() >> 8) | (sectors << 24));
						o += sectors;
						timestamp.write(last_modified);
						new_chunks.push(chunk);
					} else {
						offset.write(0);
						timestamp.write(0);
					}
				}
				writer.write(&core::mem::transmute::<_, [u8; 4096]>(offsets));
				writer.write(&core::mem::transmute::<_, [u8; 4096]>(timestamps));
				for chunk in new_chunks {
					writer.write(&chunk);
				}
			});
		}
	}

	#[inline]
	pub fn increment(&mut self, amount: usize, true_amount: usize) {
		self.height = self.height.wrapping_add(amount as u32);
		self.true_height = self.true_height.wrapping_add(true_amount as u32);
	}

	#[inline]
	pub fn decrement(&mut self, amount: usize, true_amount: usize) {
		self.height = self.height.wrapping_sub(amount as u32);
		self.true_height = self.true_height.wrapping_sub(true_amount as u32);
	}

	#[inline]
	#[must_use]
	pub const fn height(&self) -> usize {
		if self.open {
			self.height as usize
		} else {
			1
		}
	}

	#[inline]
	#[must_use]
	pub const fn true_height(&self) -> usize {
		self.true_height as usize
	}

	#[inline]
	pub fn toggle(&mut self) -> Option<()> {
		self.open = !self.open && !self.is_empty();
		if !self.open {
			self.shut();
		}
		Some(())
	}

	#[inline]
	#[must_use]
	pub const fn open(&self) -> bool {
		self.open
	}

	#[inline]
	#[must_use]
	pub fn len(&self) -> usize {
		(*self.chunks).0.len()
	}

	#[inline]
	#[must_use]
	pub fn is_empty(&self) -> bool {
		(*self.chunks).0.is_empty()
	}

	/// # Errors
	///
	/// * `NbtElement` is not of `NbtChunk`
	///
	/// * Index is outside the range of `NbtRegion`
	#[inline]
	pub fn insert(&mut self, idx: usize, mut value: NbtElement) -> Result<(), NbtElement> {
		if let Some(chunk) = value.as_chunk_mut() {
			let mut pos = ((chunk.x as u16) << 5) | (chunk.z as u16);
			let (map, chunks) = &mut *self.chunks;
			while !chunks[pos as usize].is_null() && pos < chunks.len() as u16 {
				pos += 1;
			}
			chunk.x = (pos >> 5) as u8 & 31;
			chunk.z = pos as u8 & 31;
			if pos < chunks.len() as u16 && idx <= map.len() && chunks[pos as usize].is_null() {
				let (height, true_height) = (value.height(), value.true_height());
				map.insert(idx, pos);
				chunks[map[idx] as usize] = value;
				self.increment(height, true_height);
				return Ok(());
			}
		}

		Err(value)
	}

	/// # Safety
	///
	/// * `value` must be variant `NbtElement::Chunk`
	///
	/// * `self.map` must not contain a chunk in this `pos` already
	///
	/// * `pos` is between 0..=1023
	#[inline]
	pub unsafe fn insert_unchecked(&mut self, pos: usize, idx: usize, value: NbtElement) {
		self.increment(value.height(), value.true_height());
		let (map, chunks) = &mut *self.chunks;
		map.insert(idx, pos as u16);
		unsafe {
			chunks.as_mut_ptr().cast::<NbtElement>().add(pos).write(value);
		}
	}

	#[inline]
	#[must_use]
	pub fn remove(&mut self, idx: usize) -> NbtElement {
		let (map, chunks) = &mut *self.chunks;
		unsafe { core::ptr::replace(core::ptr::addr_of_mut!(chunks[map.remove(idx) as usize]), core::mem::zeroed()) }
	}

	#[inline]
	#[must_use]
	pub fn get(&self, idx: usize) -> Option<&NbtElement> {
		let (map, chunks) = &*self.chunks;
		map.get(idx).and_then(|&x| chunks.get(x as usize))
	}

	#[inline]
	#[must_use]
	pub fn get_mut(&mut self, idx: usize) -> Option<&mut NbtElement> {
		let (map, chunks) = &mut *self.chunks;
		map.get(idx).and_then(|&x| chunks.get_mut(x as usize))
	}

	#[inline]
	#[must_use]
	pub fn value(&self) -> CompactString {
		format_compact!("{} chunk{}", self.len(), if self.len() == 1 { "" } else { "s" })
	}

	#[inline]
	#[allow(clippy::too_many_lines)]
	pub fn render_root(&self, builder: &mut VertexBufferBuilder, str: &str, ctx: &mut RenderContext) {
		use std::fmt::Write;

		let mut remaining_scroll = builder.scroll() / 16;
		'head: {
			if remaining_scroll > 0 {
				remaining_scroll -= 1;
				ctx.skip_line_numbers(1);
				break 'head;
			}

			ctx.line_number();
			// fun hack for connection
			builder.draw_texture_z((builder.text_coords.0 + 4, ctx.y_offset - 2), LINE_NUMBER_CONNECTOR_Z, LINE_NUMBER_SEPARATOR_UV, (2, 2));
			Self::render_icon(ctx.pos(), BASE_Z, builder);
			builder.draw_texture(ctx.pos() - (16, 0), CONNECTION_UV, (16, 9));
			if !self.is_empty() {
				ctx.draw_toggle(ctx.pos() - (16, 0), self.open, builder);
			}
			ctx.render_errors(ctx.pos(), builder);
			if ctx.forbid(ctx.pos()) {
				builder.settings(ctx.pos() + (20, 0), false, BASE_TEXT_Z);
				let _ = write!(builder, "{str} [{}]", self.value());
			}

			let pos = ctx.pos();
			if ctx.ghost(ctx.pos() + (16, 16), builder, |x, y| pos == (x - 16, y - 8), |id| id == NbtChunk::ID) {
				builder.draw_texture(ctx.pos() + (0, 16), CONNECTION_UV, (16, (self.height() != 1) as usize * 7 + 9));
				ctx.y_offset += 16;
			} else if self.height() == 1 && ctx.ghost(ctx.pos() + (16, 16), builder, |x, y| pos == (x - 16, y - 16), |id| id == NbtChunk::ID) {
				builder.draw_texture(ctx.pos() + (0, 16), CONNECTION_UV, (16, 9));
				ctx.y_offset += 16;
			}

			ctx.y_offset += 16;
		}

		ctx.x_offset += 16;

		if self.open {
			let shadowing_other = {
				let children_contains_forbidden = 'f: {
					let mut y = ctx.y_offset;
					for value in self.children() {
						if y.saturating_sub(remaining_scroll * 16) == ctx.forbidden_y && ctx.forbidden_y >= HEADER_SIZE {
							break 'f true;
						}
						y += value.height() * 16;
					}
					false
				};
				if children_contains_forbidden {
					let mut y = ctx.y_offset;
					'a: {
						for value in self.children() {
							let value = unsafe { value.as_chunk_unchecked() };
							let x = value.x.to_compact_string();
							let z = value.z.to_compact_string();
							ctx.check_for_key_duplicate(|key, value| key.parse::<u8>().ok() == x.parse::<u8>().ok() && value.parse::<u8>().ok() == z.parse::<u8>().ok(), true);
							// first check required so this don't render when it's the only selected
							let y2 = y.saturating_sub(remaining_scroll * 16);
							if y2 != ctx.forbidden_y && y2 >= HEADER_SIZE && ctx.key_duplicate_error {
								ctx.red_line_numbers[1] = y2;
								ctx.draw_error_underline_width(ctx.x_offset, 0, y2, x.width() + ", ".width() + z.width(), builder);
								break 'a true;
							}
							y += value.height() * 16;
						}
						false
					}
				} else {
					false
				}
			};

			for (idx, value) in self.children().enumerate() {
				let value = unsafe { value.as_chunk_unchecked() };
				if ctx.y_offset > builder.window_height() {
					break;
				}

				let height = value.height();
				if remaining_scroll >= height {
					remaining_scroll -= height;
					ctx.skip_line_numbers(value.true_height());
					continue;
				}

				let pos = ctx.pos();
				if ctx.ghost(ctx.pos(), builder, |x, y| pos == (x, y), |id| id == NbtChunk::ID) {
					builder.draw_texture(ctx.pos() - (16, 0), CONNECTION_UV, (16, 16));
					ctx.y_offset += 16;
				}

				let ghost_tail_mod = if let Some((_, x, y, _)) = ctx.ghost && ctx.pos() + (0, height * 16 - remaining_scroll * 16 - 8) == (x, y) {
					false
				} else {
					true
				};

				if remaining_scroll == 0 {
					builder.draw_texture(ctx.pos() - (16, 0), CONNECTION_UV, (16, (idx != self.len() - 1 || !ghost_tail_mod) as usize * 7 + 9));
				}
				let forbidden_y = ctx.forbidden_y;
				let pos = ctx.pos();
				ctx.check_for_key_duplicate(|_, _| shadowing_other && pos.y == forbidden_y, true);
				if ctx.key_duplicate_error {
					ctx.red_line_numbers[0] = ctx.y_offset;
				}
				value.render(builder, &mut remaining_scroll, idx == self.len() - 1 && ghost_tail_mod, ctx);

				let pos = ctx.pos();
				if ctx.ghost(ctx.pos(), builder, |x, y| pos == (x, y + 8), |id| id == NbtChunk::ID) {
					builder.draw_texture(ctx.pos() - (16, 0), CONNECTION_UV, (16, (idx != self.len() - 1) as usize * 7 + 9));
					ctx.y_offset += 16;
				}
			}
		}
	}

	#[inline]
	pub fn render_icon(pos: impl Into<(usize, usize)>, z: u8, builder: &mut VertexBufferBuilder) {
		builder.draw_texture_z(pos, z, REGION_UV, (16, 16));
	}

	#[inline]
	pub fn children(&self) -> ValueIterator {
		let (map, chunks) = &*self.chunks;
		ValueIterator::Region(chunks, map.iter())
	}

	#[inline]
	pub fn children_mut(&mut self) -> ValueMutIterator {
		let (map, chunks) = &mut *self.chunks;
		ValueMutIterator::Region(chunks, map.iter())
	}

	#[inline]
	pub fn drop(&mut self, mut key: Option<CompactString>, mut element: NbtElement, y: &mut usize, depth: usize, target_depth: usize, mut line_number: usize, indices: &mut Vec<usize>) -> DropFn {
		if *y < 16 && *y >= 8 && depth == target_depth && let Some(chunk) = element.as_chunk() {
			let (_x, z) = (chunk.x, chunk.z);
			let before = (self.height(), self.true_height());
			indices.push(0);
			if let Err(element) = self.insert(0, element) {
				return DropFn::InvalidType(key, element);
			}
			self.open = true;
			return DropFn::Dropped(self.height as usize - before.0, self.true_height as usize - before.1, Some(z.to_compact_string()), line_number + 1);
		} else if self.height() == 1 && *y < 24 && *y >= 16 && depth == target_depth && let Some(chunk) = element.as_chunk() {
			let (_x, z) = (chunk.x, chunk.z);
			let before = self.true_height();
			indices.push(self.len());
			if let Err(element) = self.insert(self.len(), element) {
				// indices are never used
				return DropFn::InvalidType(key, element);
			}
			self.open = true;
			return DropFn::Dropped(self.height as usize - 1, self.true_height as usize - before, Some(z.to_compact_string()), line_number + before + 1);
		}

		if *y < 16 {
			return DropFn::Missed(key, element);
		} else {
			*y -= 16;
		}

		if self.open && !self.is_empty() {
			indices.push(0);
			let ptr = unsafe { &mut *indices.as_mut_ptr().add(indices.len() - 1) };
			for (idx, value) in self.children_mut().enumerate() {
				*ptr = idx;
				let heights = (element.height(), element.true_height());
				if *y < 8 && depth == target_depth && let Some(chunk) = element.as_chunk() {
					let (_x, z) = (chunk.x, chunk.z);
					if let Err(element) = self.insert(idx, element) {
						return DropFn::InvalidType(key, element);
					}
					return DropFn::Dropped(heights.0, heights.1, Some(z.to_compact_string()), line_number + 1);
				} else if *y >= value.height() * 16 - 8 && *y < value.height() * 16 && depth == target_depth && let Some(chunk) = element.as_chunk() {
					let (_x, z) = (chunk.x, chunk.z);
					*ptr = idx + 1;
					let true_height = element.true_height();
					if let Err(element) = self.insert(idx + 1, element) {
						return DropFn::InvalidType(key, element);
					}
					return DropFn::Dropped(heights.0, heights.1, Some(z.to_compact_string()), line_number + true_height + 1);
				}

				if element.id() != NbtChunk::ID {
					match value.drop(key, element, y, depth + 1, target_depth, line_number, indices) {
						x @ DropFn::InvalidType(_, _) => return x,
						DropFn::Missed(k, e) => {
							key = k;
							element = e;
						}
						DropFn::Dropped(increment, true_increment, key, line_number) => {
							self.increment(increment, true_increment);
							return DropFn::Dropped(increment, true_increment, key, line_number);
						}
					}
				}

				line_number += value.true_height();
			}
			indices.pop();
		}
		DropFn::Missed(key, element)
	}

	#[inline]
	pub fn shut(&mut self) {
		for element in self.children_mut() {
			element.shut();
		}
		self.open = false;
		self.height = self.len() as u32 + 1;
	}

	#[inline]
	pub fn expand<'a, 'b>(&'b mut self, scope: &'a Scope<'a, 'b>) {
		self.open = !self.is_empty();
		self.height = self.true_height;
		let mut iter = self.children_mut().array_chunks::<{ Self::CHUNK_BANDWIDTH }>();
		for elements in iter.by_ref() {
			scope.spawn(|| {
				for element in elements {
					element.expand(scope);
				}
			});
		}
		if let Some(rem) = iter.into_remainder() {
			scope.spawn(|| {
				for element in rem {
					element.expand(scope);
				}
			});
		}
	}

	#[inline]
	pub fn recache_depth(&mut self) {
		let mut max_depth = 0;
		if self.open() {
			for child in self.children() {
				max_depth = usize::max(max_depth, 16 + 4 + child.value().0.width());
				max_depth = usize::max(max_depth, 16 + child.max_depth());
			}
		}
		self.max_depth = max_depth as u32;
	}

	#[inline]
	#[must_use]
	pub const fn max_depth(&self) -> usize {
		self.max_depth as usize
	}
}

impl Debug for NbtRegion {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let mut r#struct = f.debug_struct("Region");
		for chunk in self.children().map(|x| unsafe { x.as_chunk_unchecked() }) {
			r#struct.field(&format!("x: {:02}, z: {:02}", chunk.x, chunk.z), &chunk);
		}
		r#struct.finish()
	}
}

#[repr(C)]
#[allow(clippy::module_name_repetitions)]
pub struct NbtChunk {
	inner: Box<NbtCompound>,
	last_modified: u32,
	// need to restrict this file format to only use GZIP, ZLIB and Uncompressed
	compression: FileFormat,
	pub x: u8,
	pub z: u8,
}

impl Clone for NbtChunk {
	#[allow(clippy::cast_ptr_alignment)]
	#[inline]
	fn clone(&self) -> Self {
		unsafe {
			let boxx = alloc(Layout::new::<NbtCompound>()).cast::<NbtCompound>();
			boxx.write(self.inner.deref().clone());
			Self {
				inner: Box::from_raw(boxx),
				last_modified: self.last_modified,
				compression: self.compression,
				x: self.x,
				z: self.z,
			}
		}
	}
}

impl NbtChunk {
	pub const ID: u8 = 129;
}

impl NbtChunk {
	#[must_use]
	pub fn from_compound(compound: NbtCompound, pos: (u8, u8), compression: FileFormat, last_modified: u32) -> Self {
		Self {
			x: pos.0,
			z: pos.1,
			inner: Box::new(compound),
			compression,
			last_modified,
		}
	}
	pub fn to_bytes(&self, writer: &mut UncheckedBufWriter) {
		// todo, mcc
		unsafe {
			let encoded = self.compression.encode(&*(self.inner.as_ref() as *const NbtCompound).cast::<NbtElement>());
			let len = encoded.len() + 1;
			// plus four for the len field writing, and + 1 for the compression
			let pad_len = (4096 - (len + 4) % 4096) % 4096;
			writer.write(&(len as u32).to_be_bytes());
			writer.write(
				&match self.compression {
					FileFormat::Gzip => 1_u8,
					FileFormat::Zlib => 2_u8,
					FileFormat::Nbt => 3_u8,
					_ => core::hint::unreachable_unchecked(),
				}
				.to_be_bytes(),
			);
			writer.write(&encoded);
			drop(encoded);
			let mut pad = Box::<[u8]>::new_uninit_slice(pad_len);
			pad.as_mut_ptr().write_bytes(0, pad_len);
			writer.write(&pad.assume_init());
		}
	}

	#[inline]
	#[must_use]
	pub fn value(&self) -> String {
		format!("{}, {}", self.x, self.z)
	}

	#[inline]
	#[allow(clippy::too_many_lines)]
	pub fn render(&self, builder: &mut VertexBufferBuilder, remaining_scroll: &mut usize, tail: bool, ctx: &mut RenderContext) {
		use std::fmt::Write;

		let mut y_before = ctx.y_offset;

		'head: {
			if *remaining_scroll > 0 {
				*remaining_scroll -= 1;
				ctx.skip_line_numbers(1);
				break 'head;
			}

			let name = self.value();
			ctx.line_number();
			Self::render_icon(ctx.pos(), BASE_Z, builder);
			if !self.is_empty() {
				ctx.draw_toggle(ctx.pos() - (16, 0), self.open(), builder);
			}
			ctx.render_errors(ctx.pos(), builder);
			if ctx.forbid(ctx.pos()) {
				builder.settings(ctx.pos() + (20, 0), false, BASE_TEXT_Z);
				let _ = write!(builder, "{name}");
			}

			let pos = ctx.pos();
			if ctx.ghost(ctx.pos() + (16, 16), builder, |x, y| pos == (x - 16, y - 8), |id| id != Self::ID) {
				builder.draw_texture(ctx.pos() + (0, 16), CONNECTION_UV, (16, (self.height() != 1) as usize * 7 + 9));
				if !tail {
					builder.draw_texture(ctx.pos() - (16, 0) + (0, 16), CONNECTION_UV, (8, 16));
				}
				ctx.y_offset += 16;
			} else if self.height() == 1 && ctx.ghost(ctx.pos() + (16, 16), builder, |x, y| pos == (x - 16, y - 16), |id| id != Self::ID) {
				builder.draw_texture(ctx.pos() + (0, 16), CONNECTION_UV, (16, 9));
				if !tail {
					builder.draw_texture(ctx.pos() - (16, 0) + (0, 16), CONNECTION_UV, (8, 16));
				}
				ctx.y_offset += 16;
			}

			ctx.y_offset += 16;
			y_before += 16;
		}

		let x_before = ctx.x_offset - 16;

		if self.open() {
			ctx.x_offset += 16;

			{
				let children_contains_forbidden = 'f: {
					let mut y = ctx.y_offset;
					for (_, value) in self.children() {
						if y.saturating_sub(*remaining_scroll * 16) == ctx.forbidden_y && ctx.forbidden_y >= HEADER_SIZE {
							break 'f true;
						}
						y += value.height() * 16;
					}
					false
				};
				if children_contains_forbidden {
					let mut y = ctx.y_offset;
					for (name, value) in self.children() {
						ctx.check_for_key_duplicate(|text, _| text == name, false);
						// first check required so this don't render when it's the only selected
						if y.saturating_sub(*remaining_scroll * 16) != ctx.forbidden_y && y.saturating_sub(*remaining_scroll * 16) >= HEADER_SIZE && ctx.key_duplicate_error {
							ctx.red_line_numbers[1] = y.saturating_sub(*remaining_scroll * 16);
							ctx.draw_error_underline(ctx.x_offset, y.saturating_sub(*remaining_scroll * 16), builder);
							break;
						}
						y += value.height() * 16;
					}
				}
			}

			for (idx, (key, entry)) in self.children().enumerate() {
				if ctx.y_offset > builder.window_height() {
					break;
				}

				let height = entry.height();
				if *remaining_scroll >= height {
					*remaining_scroll -= height;
					ctx.skip_line_numbers(entry.true_height());
					continue;
				}

				let pos = ctx.pos();
				if ctx.ghost(ctx.pos(), builder, |x, y| pos == (x, y), |id| id != Self::ID) {
					builder.draw_texture(ctx.pos() - (16, 0), CONNECTION_UV, (16, 16));
					ctx.y_offset += 16;
				}

				let ghost_tail_mod = if let Some((_, x, y, _)) = ctx.ghost && ctx.pos() + (0, height * 16 - *remaining_scroll * 16 - 8) == (x, y) {
					false
				} else {
					true
				};

				if *remaining_scroll == 0 {
					builder.draw_texture(ctx.pos() - (16, 0), CONNECTION_UV, (16, (idx != self.len() - 1 || !ghost_tail_mod) as usize * 7 + 9));
				}
				ctx.check_for_key_duplicate(|text, _| self.inner.entries.has(text) && key != text, false);
				if ctx.key_duplicate_error && ctx.y_offset == ctx.forbidden_y {
					ctx.red_line_numbers[0] = ctx.y_offset;
				}
				entry.render(remaining_scroll, builder, Some(key), tail && idx == self.len() - 1 && ghost_tail_mod, ctx);

				let pos = ctx.pos();
				if ctx.ghost(ctx.pos(), builder, |x, y| pos == (x, y + 8), |id| id != Self::ID) {
					builder.draw_texture(ctx.pos() - (16, 0), CONNECTION_UV, (16, (idx != self.len() - 1) as usize * 7 + 9));
					ctx.y_offset += 16;
				}
			}

			if !tail {
				let len = (ctx.y_offset - y_before) / 16;
				for i in 0..len {
					builder.draw_texture((x_before, y_before + i * 16), CONNECTION_UV, (8, 16));
				}
			}

			ctx.x_offset -= 16;
		} else {
			ctx.skip_line_numbers(self.true_height() - 1);
		}
	}

	#[inline]
	pub fn render_icon(pos: impl Into<(usize, usize)>, z: u8, builder: &mut VertexBufferBuilder) {
		builder.draw_texture_z(pos, z, CHUNK_UV, (16, 16));
	}
}

impl Deref for NbtChunk {
	type Target = NbtCompound;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

impl DerefMut for NbtChunk {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}

impl Display for NbtChunk {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}|{}{{", self.x, self.z)?;
		for (idx, (key, value)) in self.children().enumerate() {
			if key.needs_escape() {
				write!(f, "{key:?}")?;
			} else {
				write!(f, "{key}")?;
			}
			write!(f, ":{value}")?;
			if likely(idx < self.len() - 1) {
				write!(f, ",")?;
			}
		}
		write!(f, "}}")
	}
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for NbtChunk {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} | {}", self.x, self.z)?;
		if self.is_empty() {
			write!(f, "{{}}")
		} else {
			let mut debug = f.debug_struct("");
			for (key, element) in self.children() {
				if key.needs_escape() {
					debug.field(&format!("{key:?}"), element);
				} else {
					debug.field(key, element);
				}
			}
			debug.finish()
		}
	}
}
