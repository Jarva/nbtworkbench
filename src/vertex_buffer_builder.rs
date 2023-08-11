use winit::dpi::PhysicalSize;

use crate::assets;

pub struct VertexBufferBuilder {
	vertices: Vec<f32>,
	indices: Vec<u16>,
	text_vertices: Vec<f32>,
	text_indices: Vec<u16>,
	vertices_len: u32,
	text_vertices_len: u32,
	window_width: f32,
	window_height: f32,
	recip_texture_width: f32,
	recip_texture_height: f32,
	scroll: usize,
	pub horizontal_scroll: usize,
	pub text_coords: (usize, usize),
	dropshadow: bool,
	pub color: usize,
	two_over_width: f32,
	negative_two_over_height: f32,
}

impl core::fmt::Write for VertexBufferBuilder {
	fn write_str(&mut self, text: &str) -> std::fmt::Result {
		let (mut x, y) = self.text_coords;
		x += text
			.chars()
			.fold(0, |offset, char| offset + if (char as u32) < 56832 { self.draw_char(char as u16, x + offset, y, 0.0) } else { 0 });
		self.text_coords = (x, y);
		Ok(())
	}

	fn write_char(&mut self, c: char) -> std::fmt::Result {
		if (c as u32) < 56832 {
			self.text_coords.0 += self.draw_char(c as u16, self.text_coords.0, self.text_coords.1, 0.0);
		}
		Ok(())
	}
}

impl VertexBufferBuilder {
	pub const CHAR_WIDTH: &'static [u8] = include_bytes!("assets/char_widths.hex");

	pub fn new(size: PhysicalSize<u32>, texture_width: usize, texture_height: usize, scroll: usize) -> Self {
		Self {
			vertices: Vec::with_capacity(98304),
			indices: Vec::with_capacity(65536),
			text_vertices: Vec::with_capacity(98304),
			text_indices: Vec::with_capacity(65536),
			vertices_len: 0,
			text_vertices_len: 0,
			window_width: size.width as f32,
			window_height: size.height as f32,
			recip_texture_width: (texture_width as f32).recip(),
			recip_texture_height: (texture_height as f32).recip(),
			scroll,
			horizontal_scroll: 0,
			text_coords: (0, 0),
			dropshadow: false,
			color: 0xFF,
			two_over_width: 2.0 / size.width as f32,
			negative_two_over_height: -2.0 / size.height as f32,
		}
	}

	#[inline]
	pub const fn scroll(&self) -> usize {
		self.scroll
	}

	#[inline]
	pub const fn furthest_pixel(char: u16) -> usize {
		let mut x_pixel = 15;
		while x_pixel > 0 {
			let mut y_pixel = 15;
			while y_pixel > 0 {
				if ((assets::UNICODE[char as usize * 32 + y_pixel * 2 + x_pixel / 8] >> (7 - x_pixel % 8)) & 1) == 1 {
					return x_pixel + 2;
				}
				y_pixel -= 1;
			}
			x_pixel -= 1;
		}
		5 // space
	}

	#[inline]
	pub fn settings(&mut self, x: usize, y: usize, dropshadow: bool) {
		self.text_coords = (x, y);
		self.dropshadow = dropshadow;
	}

	#[inline]
	fn draw_char(&mut self, c: u16, x: usize, y: usize, z: f32) -> usize {
		if self.dropshadow {
			self.draw_unicode_z_color(x + 1, y + 1, z, c, (self.color * 21 / 85) as u8);
		}
		self.draw_unicode_z_color(x, y, z, c, self.color as u8);
		Self::CHAR_WIDTH[c as usize] as usize
	}

	#[inline]
	pub fn draw_unicode_z_color(&mut self, x: usize, y: usize, z: f32, char: u16, color: u8) {
		unsafe {
			let x = (x as isize - self.horizontal_scroll as isize) as f32;
			let y = y as f32;
			let z = z;
			let char = *(&((char as u32) | ((color as u32) << 24)) as *const u32).cast::<f32>();

			let x0 = x.mul_add(self.two_over_width, -1.0);
			let x1 = self.two_over_width.mul_add(16.0, x0);
			let y1 = y.mul_add(self.negative_two_over_height, 1.0);
			let y0 = self.negative_two_over_height.mul_add(16.0, y1);

			let len = self.text_vertices_len;
			let vec = &mut self.text_vertices;

			let vertices_len = vec.len();
			let ptr = vec.as_mut_ptr().add(vertices_len);
			// top left, 0 -> 1.0, 0.0
			*ptr = x1;
			*(ptr.add(1)) = y1;
			*(ptr.add(2)) = z;
			*(ptr.add(3)) = char;
			// top right, 1 -> 0.0, 0.0
			*(ptr.add(4)) = x0;
			*(ptr.add(5)) = y1;
			*(ptr.add(6)) = z;
			*(ptr.add(7)) = char;
			// bottom left, 2 -> 0.0, 1.0
			*(ptr.add(8)) = x0;
			*(ptr.add(9)) = y0;
			*(ptr.add(10)) = z;
			*(ptr.add(11)) = char;
			// bottom right, 3 -> 1.0, 1.0
			*(ptr.add(12)) = x1;
			*(ptr.add(13)) = y0;
			*(ptr.add(14)) = z;
			*(ptr.add(15)) = char;

			vec.set_len(vertices_len + 16);

			let indices_len = self.text_indices.len();
			let ptr = self.text_indices.as_mut_ptr().add(indices_len).cast::<u8>();

			*ptr = len as u8;
			*(ptr.add(1)) = (len >> 8) as u8;
			*(ptr.add(2)) = (len + 1) as u8;
			*(ptr.add(3)) = ((len + 1) >> 8) as u8;
			*(ptr.add(4)) = (len + 2) as u8;
			*(ptr.add(5)) = ((len + 2) >> 8) as u8;
			*(ptr.add(6)) = *ptr;
			*(ptr.add(7)) = *(ptr.add(1));
			*(ptr.add(8)) = *(ptr.add(4));
			*(ptr.add(9)) = *(ptr.add(5));
			*(ptr.add(10)) = (len + 3) as u8;
			*(ptr.add(11)) = ((len + 3) >> 8) as u8;

			self.text_indices.set_len(indices_len + 6);

			self.text_vertices_len += 4;
		}
	}

	#[inline]
	pub const fn window_height(&self) -> usize {
		self.window_height as usize
	}

	#[inline]
	pub const fn window_width(&self) -> usize {
		self.window_width as usize
	}

	#[inline]
	pub fn vertices(&self) -> &[u8] {
		unsafe { core::slice::from_raw_parts(self.vertices.as_ptr().cast::<u8>(), self.vertices.len() * 4) }
	}

	#[inline]
	pub fn indices(&self) -> &[u8] {
		unsafe { core::slice::from_raw_parts(self.indices.as_ptr().cast::<u8>(), self.indices.len() * 4) }
	}

	#[inline]
	pub fn text_vertices(&self) -> &[u8] {
		unsafe { core::slice::from_raw_parts(self.text_vertices.as_ptr().cast::<u8>(), self.text_vertices.len() * 4) }
	}

	#[inline]
	pub fn text_indices(&self) -> &[u8] {
		unsafe { core::slice::from_raw_parts(self.text_indices.as_ptr().cast::<u8>(), self.text_indices.len() * 4) }
	}

	#[inline]
	pub fn indices_len(&self) -> u32 {
		self.indices.len() as u32
	}

	#[inline]
	pub fn text_indices_len(&self) -> u32 {
		self.text_indices.len() as u32
	}

	#[inline]
	pub fn draw_texture(&mut self, pos: (usize, usize), uv: (usize, usize), dims: (usize, usize)) {
		self.draw_texture_z(pos, 0.0, uv, dims);
	}

	#[inline]
	pub fn draw_texture_z(&mut self, pos: (usize, usize), z: f32, uv: (usize, usize), dims: (usize, usize)) {
		self.draw_texture_region_z(pos, z, uv, dims, dims);
	}

	#[inline]
	#[allow(clippy::many_single_char_names)]
	pub fn draw_texture_region_z(&mut self, pos: (usize, usize), z: f32, uv: (usize, usize), dims: (usize, usize), uv_dims: (usize, usize)) {
		unsafe {
			let x = (pos.0 as isize - self.horizontal_scroll as isize) as f32;
			let y = pos.1 as f32;
			let u = uv.0 as f32;
			let v = uv.1 as f32;
			let width = dims.0 as f32;
			let height = dims.1 as f32;
			let uv_width = uv_dims.0 as f32;
			let uv_height = uv_dims.1 as f32;

			let x0 = self.two_over_width.mul_add(x, -1.0);
			let y1 = self.negative_two_over_height.mul_add(y, 1.0);
			let u0 = self.recip_texture_width * u;
			let v0 = self.recip_texture_height * v;
			let x1 = self.two_over_width.mul_add(width, x0);
			let y0 = self.negative_two_over_height.mul_add(height, y1);
			let u1 = self.recip_texture_width.mul_add(uv_width, u0);
			let v1 = self.recip_texture_height.mul_add(uv_height, v0);
			let z = z;

			let len = self.vertices_len;
			let vec = &mut self.vertices;

			let vertices_len = vec.len();
			let ptr = vec.as_mut_ptr().add(vertices_len);
			// top left
			*ptr = x1;
			*(ptr.add(1)) = y1;
			*(ptr.add(2)) = z;
			*(ptr.add(3)) = u1;
			*(ptr.add(4)) = v0;
			// top right
			*(ptr.add(5)) = x0;
			*(ptr.add(6)) = y1;
			*(ptr.add(7)) = z;
			*(ptr.add(8)) = u0;
			*(ptr.add(9)) = v0;
			// bottom left
			*(ptr.add(10)) = x0;
			*(ptr.add(11)) = y0;
			*(ptr.add(12)) = z;
			*(ptr.add(13)) = u0;
			*(ptr.add(14)) = v1;
			// bottom right
			*(ptr.add(15)) = x1;
			*(ptr.add(16)) = y0;
			*(ptr.add(17)) = z;
			*(ptr.add(18)) = u1;
			*(ptr.add(19)) = v1;

			vec.set_len(vertices_len + 20);

			let indices_len = self.indices.len();
			let ptr = self.indices.as_mut_ptr().add(indices_len).cast::<u8>();

			*ptr = len as u8;
			*(ptr.add(1)) = (len >> 8) as u8;
			*(ptr.add(2)) = (len + 1) as u8;
			*(ptr.add(3)) = ((len + 1) >> 8) as u8;
			*(ptr.add(4)) = (len + 2) as u8;
			*(ptr.add(5)) = ((len + 2) >> 8) as u8;
			*(ptr.add(6)) = *ptr;
			*(ptr.add(7)) = *(ptr.add(1));
			*(ptr.add(8)) = *(ptr.add(4));
			*(ptr.add(9)) = *(ptr.add(5));
			*(ptr.add(10)) = (len + 3) as u8;
			*(ptr.add(11)) = ((len + 3) >> 8) as u8;

			self.indices.set_len(indices_len + 6);

			self.vertices_len += 4;
		}
	}
}
