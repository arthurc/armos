use std::{
    io::{self, Read, Seek, SeekFrom},
    marker::PhantomData,
    mem,
};

use byteorder::{BigEndian, ReadBytesExt};

pub enum Segment {
    Scene = 2,
    Room = 3,
    Keep = 4,
    FieldDungeonKeep = 5,
    Object = 6,
    LinkAnimation = 7,
}

type E = BigEndian;

const fn segment_number(addr: u32) -> u32 {
    (addr << 4) >> 28
}

const fn segment_offset(addr: u32) -> u32 {
    addr & 0x00FFFFFF
}

pub trait ReadSegment<R> {
    const SIZE: usize;

    fn read(r: &mut RomReader<R>) -> io::Result<Self>
    where
        Self: Sized;
}

pub struct RomReader<R> {
    r: R,
    segments: [u32; 8],
}
impl<R> RomReader<R> {
    pub fn new(r: R) -> Self {
        Self {
            r,
            segments: [0; 8],
        }
    }

    pub fn with_segment(mut self, segment: Segment, address: u32) -> Self {
        self.segments[segment as usize] = address;
        self
    }

    pub fn ptr_segment_iter<T>(&mut self, addr: u32) -> PtrSegmentIter<R, T>
    where
        T: ReadSegment<R>,
    {
        PtrSegmentIter::new(self, addr)
    }

    pub fn segment_iter<T>(&mut self, addr: u32) -> SegmentIter<R, T>
    where
        T: ReadSegment<R>,
    {
        SegmentIter::new(self, addr)
    }
}
impl<R> RomReader<R>
where
    R: Read,
{
    pub fn read_u8(&mut self) -> io::Result<u8> {
        self.r.read_u8()
    }

    pub fn read_u16(&mut self) -> io::Result<u16> {
        self.r.read_u16::<E>()
    }

    pub fn read_u32(&mut self) -> io::Result<u32> {
        self.r.read_u32::<E>()
    }

    pub fn read_i16(&mut self) -> io::Result<i16> {
        self.r.read_i16::<E>()
    }

    pub fn read_i32(&mut self) -> io::Result<i32> {
        self.r.read_i32::<E>()
    }

    pub fn read_segment<T>(&mut self) -> io::Result<T>
    where
        T: ReadSegment<R>,
    {
        T::read(self)
    }
}
impl<R> RomReader<R>
where
    R: Seek,
{
    pub fn seek(&mut self, offset: u64) -> io::Result<u64> {
        self.r.seek(SeekFrom::Start(
            (self.segments[segment_number(offset as _) as usize] + segment_offset(offset as _))
                as u64,
        ))
    }
}

pub struct PtrSegmentIter<'a, R, T> {
    r: &'a mut RomReader<R>,
    pos: u64,
    _marker: PhantomData<T>,
}
impl<'a, R, T> PtrSegmentIter<'a, R, T> {
    fn new(r: &'a mut RomReader<R>, addr: u32) -> Self {
        Self {
            pos: addr as u64,
            r,
            _marker: PhantomData,
        }
    }
}
impl<R, T> Iterator for PtrSegmentIter<'_, R, T>
where
    R: Seek + Read,
    T: ReadSegment<R>,
{
    type Item = io::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // Seek to pointer table
        self.r.seek(self.pos).ok()?;

        // Read the next pointer and seek to it,
        // i.e. where the data is stored
        let addr = self.r.read_u32().ok()?;
        self.r.seek(addr as _).ok()?;

        self.pos += mem::size_of::<u32>() as u64;

        Some(T::read(self.r))
    }
}

pub struct SegmentIter<'a, R, T> {
    r: &'a mut RomReader<R>,
    pos: u64,
    _marker: PhantomData<T>,
}
impl<'a, R, T> SegmentIter<'a, R, T> {
    fn new(r: &'a mut RomReader<R>, addr: u32) -> Self {
        Self {
            pos: addr as u64,
            r,
            _marker: PhantomData,
        }
    }
}
impl<R, T> Iterator for SegmentIter<'_, R, T>
where
    R: Seek,
    T: ReadSegment<R>,
{
    type Item = io::Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.r.seek(self.pos).ok()?;
        self.pos += T::SIZE as u64;

        Some(T::read(self.r))
    }
}
