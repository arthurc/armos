use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::Add,
};

use zerocopy::{BigEndian, FromBytes, U32};

pub struct _PhysAddr(u32);

#[derive(Copy, Clone, Default, FromBytes)]
pub struct RawVirtAddr(U32<BigEndian>);
impl RawVirtAddr {
    pub fn new(n: u32) -> Self {
        Self(n.into())
    }

    pub fn segment_number(&self) -> u32 {
        (self.0.get() << 4) >> 28
    }

    pub fn segment_offset(&self) -> u32 {
        self.0.get() & 0x00FFFFFF
    }
}
impl Display for RawVirtAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#010X}", self.0)
    }
}
impl Debug for RawVirtAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [segment:{}, offset:{:06X}]",
            self,
            self.segment_number(),
            self.segment_offset()
        )
    }
}
impl Add<i32> for RawVirtAddr {
    type Output = RawVirtAddr;

    fn add(self, rhs: i32) -> Self::Output {
        Self(((self.0.get() as i64 + rhs as i64) as u32).into())
    }
}

#[derive(Default, FromBytes)]
pub struct VirtAddr<T>(RawVirtAddr, PhantomData<T>);
impl<T> VirtAddr<T> {}
impl<T> Clone for VirtAddr<T> {
    fn clone(&self) -> VirtAddr<T> {
        Self(self.0, PhantomData)
    }
}
impl<T> Copy for VirtAddr<T> {}
impl<T> Display for VirtAddr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
impl<T> Debug for VirtAddr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}
impl<T> Add<i32> for VirtAddr<T> {
    type Output = VirtAddr<T>;

    fn add(self, rhs: i32) -> Self::Output {
        Self(self.0 + rhs * std::mem::size_of::<T>() as i32, PhantomData)
    }
}
impl<T> From<RawVirtAddr> for VirtAddr<T> {
    fn from(value: RawVirtAddr) -> Self {
        Self(value, PhantomData)
    }
}
impl<T> From<VirtAddr<T>> for RawVirtAddr {
    fn from(value: VirtAddr<T>) -> Self {
        value.0
    }
}
