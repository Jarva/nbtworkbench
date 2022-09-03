use std::slice::Iter;

use crate::elements::byte::NbtByte;
use crate::elements::byte_array::NbtByteArray;
use crate::elements::compound::NbtCompound;
use crate::elements::double::NbtDouble;
use crate::elements::element_type::NbtElement::*;
use crate::elements::float::NbtFloat;
use crate::elements::int::NbtInt;
use crate::elements::int_array::NbtIntArray;
use crate::elements::list::NbtList;
use crate::elements::long::NbtLong;
use crate::elements::long_array::NbtLongArray;
use crate::elements::short::NbtShort;
use crate::elements::string::NbtString;
use crate::{elements, VertexBufferBuilder};

#[repr(C)]
#[repr(u8)]
pub enum NbtElement {
    Null,
    Byte(NbtByte),
    Short(NbtShort),
    Int(NbtInt),
    Long(NbtLong),
    Float(NbtFloat),
    Double(NbtDouble),
    ByteArray(NbtByteArray),
    String(NbtString),
    List(NbtList),
    Compound(NbtCompound),
    IntArray(NbtIntArray),
    LongArray(NbtLongArray),
}

impl NbtElement {
    pub fn from_bytes(element: &u8, iter: &mut Iter<u8>) -> Option<Self> {
        match element {
            0 => Some(Null),
            1 => Some(Byte(NbtByte::from_bytes(iter)?)),
            2 => Some(Short(NbtShort::from_bytes(iter)?)),
            3 => Some(Int(NbtInt::from_bytes(iter)?)),
            4 => Some(Long(NbtLong::from_bytes(iter)?)),
            5 => Some(Float(NbtFloat::from_bytes(iter)?)),
            6 => Some(Double(NbtDouble::from_bytes(iter)?)),
            7 => Some(ByteArray(NbtByteArray::from_bytes(iter)?)),
            8 => Some(String(NbtString::from_bytes(iter)?)),
            9 => Some(List(NbtList::from_bytes(iter)?)),
            10 => Some(Compound(NbtCompound::from_bytes(iter)?)),
            11 => Some(IntArray(NbtIntArray::from_bytes(iter)?)),
            12 => Some(LongArray(NbtLongArray::from_bytes(iter)?)),
            _ => None
        }
    }

    pub fn to_bytes(&self, writer: &mut Vec<u8>) {
        match self {
            Null => writer.push(0),
            Byte(byte) => byte.to_bytes(writer),
            Short(short) => short.to_bytes(writer),
            Int(int) => int.to_bytes(writer),
            Long(long) => long.to_bytes(writer),
            Float(float) => float.to_bytes(writer),
            Double(double) => double.to_bytes(writer),
            ByteArray(bytes) => bytes.to_bytes(writer),
            String(string) => string.to_bytes(writer),
            List(list) => list.to_bytes(writer),
            Compound(compound) => compound.to_bytes(writer),
            IntArray(ints) => ints.to_bytes(writer),
            LongArray(longs) => longs.to_bytes(writer),
        }
    }

    #[inline]
    pub fn id(&self) -> u8 {
        match self {
            Null => 0,
            Byte(_) => 1,
            Short(_) => 2,
            Int(_) => 3,
            Long(_) => 4,
            Float(_) => 5,
            Double(_) => 6,
            ByteArray(_) => 7,
            String(_) => 8,
            List(_) => 9,
            Compound(_) => 10,
            IntArray(_) => 11,
            LongArray(_) => 12,
        }
    }

    #[inline]
    pub fn from_id(id: u8) -> NbtElement {
        match id {
            0 => Null,
            1 => Byte(NbtByte::new(0)),
            2 => Short(NbtShort::new(0)),
            3 => Int(NbtInt::new(0)),
            4 => Long(NbtLong::new(0)),
            5 => Float(NbtFloat::new(0.0)),
            6 => Double(NbtDouble::new(0.0)),
            7 => ByteArray(NbtByteArray::new(Vec::new())),
            8 => String(NbtString::new("".to_string())),
            9 => List(NbtList::new(Vec::new(), 0xFF)),
            10 => Compound(NbtCompound::new()),
            11 => IntArray(NbtIntArray::new(Vec::new())),
            _ => LongArray(NbtLongArray::new(Vec::new()))
        }
    }

    #[inline]
    pub fn from_file(bytes: &[u8]) -> Option<Self> {
        let mut iter = bytes.iter();
        iter.next();
        iter.next();
        iter.next();
        Some(Compound(NbtCompound::from_bytes(&mut iter)?))
    }

    #[inline]
    pub fn to_file(&self) -> Vec<u8> {
        let mut writer = Vec::new();
        writer.push(0x0A);
        writer.push(0x00);
        writer.push(0x00);
        self.to_bytes(&mut writer);
        writer
    }

    #[inline]
    pub fn render(&self, x: &mut u32, y: &mut u32, remaining_scroll: &mut u32, builder: &mut VertexBufferBuilder, str: Option<&str>, tail: bool) {
        match self {
            Null => *x += 8,
            Byte(byte) => byte.render(builder, x, y, str),
            Short(short) => short.render(builder, x, y, str),
            Int(int) => int.render(builder, x, y, str),
            Long(long) => long.render(builder, x, y, str),
            Float(float) => float.render(builder, x, y, str),
            Double(double) => double.render(builder, x, y, str),
            ByteArray(byte_array) => byte_array.render(builder, x, y, str, remaining_scroll, tail),
            String(string) => string.render(builder, x, y, str),
            List(list) => list.render(builder, x, y, str, remaining_scroll, tail),
            Compound(compound) => compound.render(builder, x, y, str, remaining_scroll, tail),
            IntArray(int_array) => int_array.render(builder, x, y, str, remaining_scroll, tail),
            LongArray(long_array) => long_array.render(builder, x, y, str, remaining_scroll, tail),
        }
    }

    #[inline]
    pub fn render_icon(id: u8, x: u32, y: u32, builder: &mut VertexBufferBuilder) {
        match id {
            0 => {}
            1 => elements::byte::render_icon(x, y, builder),
            2 => elements::short::render_icon(x, y, builder),
            3 => elements::int::render_icon(x, y, builder),
            4 => elements::long::render_icon(x, y, builder),
            5 => elements::float::render_icon(x, y, builder),
            6 => elements::double::render_icon(x, y, builder),
            7 => elements::byte_array::render_icon(x, y, builder),
            8 => elements::string::render_icon(x, y, builder),
            9 => elements::list::render_icon(x, y, builder),
            10 => elements::compound::render_icon(x, y, builder),
            11 => elements::int_array::render_icon(x, y, builder),
            _ => elements::long_array::render_icon(x, y, builder)
        }
    }

    #[inline]
    pub fn height(&self) -> u32 {
        match self {
            ByteArray(bytes) => bytes.height(),
            List(list) => list.height(),
            Compound(compound) => compound.height(),
            IntArray(ints) => ints.height(),
            LongArray(longs) => longs.height(),
            _ => 1
        }
    }

    #[inline]
    pub fn stack<F: FnMut(&mut NbtElement), G: FnMut(&mut NbtElement, u32, u32)>(&mut self, y: &mut u32, depth: &mut u32, index: u32, parent: &mut F, tail: &mut G) -> bool {
        if *y >= self.height() {
            *y -= self.height();
            false
        } else {
            match self {
                ByteArray(_) => NbtByteArray::stack(self, y, depth, index, parent, tail),
                LongArray(_) => NbtLongArray::stack(self, y, depth, index, parent, tail),
                IntArray(_) => NbtIntArray::stack(self, y, depth, index, parent, tail),
                List(_) => NbtList::stack(self, y, depth, index, parent, tail),
                Compound(_) => NbtCompound::stack(self, y, depth, index, parent, tail),
                x => {
                    tail(x, *depth, index);
                    true
                }
            }
        }
    }

    #[inline]
    pub fn delete(&mut self, index: u32) {
        match self {
            ByteArray(bytes) => bytes.delete(index),
            List(list) => list.delete(index),
            Compound(compound) => compound.delete(index),
            IntArray(ints) => ints.delete(index),
            LongArray(longs) => longs.delete(index),
            _ => {}
        }
    }

    #[inline]
    pub fn toggle(&mut self) -> bool {
        match self {
            ByteArray(bytes) => bytes.toggle(),
            List(list) => list.toggle(),
            Compound(compound) => compound.toggle(),
            IntArray(ints) => ints.toggle(),
            LongArray(longs) => longs.toggle(),
            _ => false
        }
    }

    #[inline]
    pub fn open(&self) -> bool {
        match self {
            ByteArray(bytes) => bytes.open(),
            List(list) => list.open(),
            Compound(compound) => compound.open(),
            IntArray(ints) => ints.open(),
            LongArray(longs) => longs.open(),
            _ => false
        }
    }

    #[inline]
    pub fn increment(&mut self, amount: u32) {
        match self {
            ByteArray(bytes) => bytes.increment(amount),
            List(list) => list.increment(amount),
            Compound(compound) => compound.increment(amount),
            IntArray(ints) => ints.increment(amount),
            LongArray(longs) => longs.increment(amount),
            _ => {}
        }
    }

    #[inline]
    pub fn decrement(&mut self, amount: u32) {
        match self {
            ByteArray(bytes) => bytes.decrement(amount),
            List(list) => list.decrement(amount),
            Compound(compound) => compound.decrement(amount),
            IntArray(ints) => ints.decrement(amount),
            LongArray(longs) => longs.decrement(amount),
            _ => {}
        }
    }

    #[inline]
    pub fn drop(&mut self, other: Self) -> bool {
        match self {
            ByteArray(bytes) => bytes.drop(other),
            List(list) => list.drop(other),
            Compound(compound) => compound.drop(other),
            IntArray(ints) => ints.drop(other),
            LongArray(longs) => longs.drop(other),
            _ => false
        }
    }
}

impl ToString for NbtElement {
    fn to_string(&self) -> std::string::String {
        match self {
            Null => "null".to_string(),
            Byte(byte) => byte.to_string(),
            Short(short) => short.to_string(),
            Int(int) => int.to_string(),
            Long(long) => long.to_string(),
            Float(float) => float.to_string(),
            Double(double) => double.to_string(),
            ByteArray(bytes) => bytes.to_string(),
            String(string) => string.to_string(),
            List(list) => list.to_string(),
            Compound(compound) => compound.to_string(),
            IntArray(ints) => ints.to_string(),
            LongArray(longs) => longs.to_string()
        }
    }
}
