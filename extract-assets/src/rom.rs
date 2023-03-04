use std::{
    fmt,
    io::{self, Read, Seek, SeekFrom},
    marker::PhantomData,
    mem, ops,
};

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};

pub enum Segment {
    Scene = 2,
    Room = 3,
    Keep = 4,
    FieldDungeonKeep = 5,
    Object = 6,
    LinkAnimation = 7,
    IconItemStatic = 8,
}

type E = BigEndian;
pub type Endian = E;

pub trait ReadSegment {
    const SIZE: u32;

    fn read(r: &mut RomReader) -> Result<Self>
    where
        Self: Sized;
}

pub struct RomReader {
    pos: VirtualAddress,
    segments: [Option<Vec<u8>>; 16],
}
impl RomReader {
    pub fn new() -> Self {
        Self {
            pos: VirtualAddress::NULL,
            segments: Default::default(),
        }
    }

    pub fn set_segment(&mut self, segment: Segment, data: Option<Vec<u8>>) {
        self.segments[segment as usize] = data;
    }

    pub fn set_segment_from(
        &mut self,
        segment: Segment,
        mut r: (impl Read + Seek),
        range: (u32, u32),
    ) -> Result<()> {
        let mut buf = vec![0u8; (range.1 - range.0) as usize];
        r.seek(SeekFrom::Start(range.0 as _))?;
        r.read_exact(&mut buf)?;

        self.set_segment(segment, Some(buf));
        Ok(())
    }

    pub fn ptr_segment_iter<T>(&mut self, addr: VirtualAddress) -> PtrSegmentIter<T>
    where
        T: ReadSegment,
    {
        PtrSegmentIter::new(self, addr)
    }

    pub fn segment_iter<T>(&mut self, addr: VirtualAddress) -> SegmentIter<T>
    where
        T: ReadSegment,
    {
        SegmentIter::new(self, addr)
    }

    pub fn seek(&mut self, addr: VirtualAddress) -> &mut Self {
        self.pos = addr;
        self
    }

    pub fn read_u8(&mut self) -> io::Result<u8> {
        ReadBytesExt::read_u8(self)
    }

    pub fn read_u16(&mut self) -> io::Result<u16> {
        ReadBytesExt::read_u16::<E>(self)
    }

    pub fn read_u32(&mut self) -> io::Result<u32> {
        ReadBytesExt::read_u32::<E>(self)
    }

    pub fn read_u64(&mut self) -> io::Result<u64> {
        ReadBytesExt::read_u64::<E>(self)
    }

    pub fn read_i8(&mut self) -> io::Result<i8> {
        ReadBytesExt::read_i8(self)
    }

    pub fn read_i16(&mut self) -> io::Result<i16> {
        ReadBytesExt::read_i16::<E>(self)
    }

    pub fn read_i32(&mut self) -> io::Result<i32> {
        ReadBytesExt::read_i32::<E>(self)
    }

    pub fn read_segment<T>(&mut self) -> Result<T>
    where
        T: ReadSegment,
    {
        Ok(T::read(self)?)
    }

    pub fn read_addr(&mut self) -> Result<VirtualAddress> {
        Ok(VirtualAddress::from(self.read_u32()?))
    }

    fn current_segment(&self) -> io::Result<&[u8]> {
        let number = self.pos.segment_number();
        let offset = self.pos.segment_offset();

        let data = self.segments[number as usize]
            .as_ref()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;

        Ok(&data[offset as usize..])
    }
}

impl Read for RomReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self
            .current_segment()
            .expect("Unexpected segment")
            .read(buf)?;
        self.pos += n as u32;
        Ok(n)
    }
}

pub struct PtrSegmentIter<'a, T> {
    r: &'a mut RomReader,
    pos: VirtualAddress,
    _marker: PhantomData<T>,
}
impl<'a, T> PtrSegmentIter<'a, T> {
    fn new(r: &'a mut RomReader, addr: VirtualAddress) -> Self {
        Self {
            r,
            pos: addr,
            _marker: PhantomData,
        }
    }
}
impl<T> Iterator for PtrSegmentIter<'_, T>
where
    T: ReadSegment,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // Seek to pointer table
        self.r.seek(self.pos);

        // Read the next pointer and seek to it,
        // i.e. where the data is stored
        let addr = self.r.read_addr().ok()?;
        log::trace!("Reading pointer segment @ {}->{}", self.pos, addr);
        self.r.seek(addr);

        self.pos += mem::size_of::<u32>() as u32;

        Some(T::read(self.r))
    }
}

pub struct SegmentIter<'a, T> {
    r: &'a mut RomReader,
    addr: VirtualAddress,
    _marker: PhantomData<T>,
}
impl<'a, T> SegmentIter<'a, T> {
    fn new(r: &'a mut RomReader, addr: VirtualAddress) -> Self {
        Self {
            addr,
            r,
            _marker: PhantomData,
        }
    }
}
impl<T> Iterator for SegmentIter<'_, T>
where
    T: ReadSegment,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.r.seek(self.addr);
        log::trace!("Reading segment @ {}", self.addr);
        self.addr += T::SIZE;

        Some(T::read(self.r))
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct VirtualAddress(u32);
impl VirtualAddress {
    pub const NULL: VirtualAddress = VirtualAddress(0);

    pub const fn segment_number(&self) -> u32 {
        (self.0 << 4) >> 28
    }

    pub const fn segment_offset(&self) -> u32 {
        self.0 & 0x00FFFFFF
    }
}
impl fmt::Display for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08X}", self.0)
    }
}
impl fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl From<u32> for VirtualAddress {
    fn from(item: u32) -> Self {
        Self(item)
    }
}
impl ops::Add<i32> for VirtualAddress {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        Self(((self.0 as i32) + rhs) as u32)
    }
}
impl<T> ops::AddAssign<T> for VirtualAddress
where
    T: Into<u32>,
{
    fn add_assign(&mut self, rhs: T) {
        self.0 += rhs.into()
    }
}
