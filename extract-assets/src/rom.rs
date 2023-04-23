use std::{
    fmt::Debug,
    io,
    ops::{Deref, Range},
};

use anyhow::{Context, Result};
use num_derive::FromPrimitive;
use zerocopy::{AsBytes, BigEndian, FromBytes, LayoutVerified};

use crate::addr::{RawVirtAddr, VirtAddr};

pub enum Segment {
    _Scene = 2,
    _Room = 3,
    _Keep = 4,
    _FieldDungeonKeep = 5,
    Object = 6,
    _LinkAnimation = 7,
    IconItemStatic = 8,
}

#[derive(FromPrimitive)]
pub enum SkinLimbType {
    Animated = 4,
    Normal = 11,
}

#[derive(Default, Clone)]
pub struct Reader {
    segments: [Option<Vec<u8>>; 16],
}
impl Reader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_segment<R: io::Read + io::Seek>(
        &mut self,
        segment: Segment,
        r: &mut R,
        range: Range<u32>,
    ) -> Result<()> {
        let mut buf = vec![0u8; range.len()];
        r.seek(io::SeekFrom::Start(range.start as u64))?;
        r.read_exact(&mut buf)?;

        self.set_segment(segment, Some(buf));

        Ok(())
    }

    pub fn set_segment(&mut self, segment: Segment, data: Option<Vec<u8>>) {
        self.segments[segment as usize] = data;
    }

    pub fn read<T>(&self, addr: VirtAddr<T>) -> Result<T>
    where
        T: FromBytes,
    {
        log::trace!("Reading struct at {}", addr);
        let (lv, _) = LayoutVerified::<_, T>::new_from_prefix(self.slice_from(addr.into())?)
            .with_context(|| format!("Failed to read item from address {}", addr))?;

        Ok(lv.read())
    }

    pub fn read_slice<T>(&self, addr: VirtAddr<T>, count: usize) -> Result<&[T]>
    where
        T: FromBytes,
    {
        log::trace!("Reading slice of count {} at {}", count, addr);
        let (lv, _) =
            LayoutVerified::<_, [T]>::new_slice_from_prefix(self.slice_from(addr.into())?, count)
                .with_context(|| format!("Failed to read slice at {}", addr))?;

        Ok(lv.into_slice())
    }

    pub fn ptr_slice_iter<'a, T>(
        &'a self,
        addr: VirtAddr<VirtAddr<T>>,
        count: usize,
    ) -> Result<impl Iterator<Item = T> + 'a>
    where
        T: FromBytes + 'a,
    {
        self.read_slice(addr, count).map(|addrs| {
            addrs
                .into_iter()
                .flat_map(|addr| self.read::<T>(*addr).into_iter())
        })
    }

    pub fn slice_from(&self, addr: RawVirtAddr) -> Result<&[u8]> {
        let number = addr.segment_number();
        let offset = addr.segment_offset();

        self.segments[number as usize]
            .as_ref()
            .map(|data| &data[offset as usize..])
            .with_context(|| format!("Segment {} has not been set", number))
    }
}

type U16 = zerocopy::U16<BigEndian>;
const _: () = assert!(std::mem::size_of::<U16>() == 0x02);
type I16 = zerocopy::I16<BigEndian>;
const _: () = assert!(std::mem::size_of::<I16>() == 0x02);
type I32 = zerocopy::I32<BigEndian>;
const _: () = assert!(std::mem::size_of::<I32>() == 0x04);

type Gfx = RawVirtAddr;

#[derive(FromBytes)]
#[repr(C, align(4))]
pub struct Aligned4<T>(T);
impl<T> Debug for Aligned4<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl<T> Deref for Aligned4<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(FromBytes)]
#[repr(C, align(2))]
pub struct Aligned2<T>(T);
impl<T> Debug for Aligned2<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl<T> Deref for Aligned2<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, FromBytes)]
#[repr(C)]
pub struct SkeletonHeader {
    pub limbs: VirtAddr<VirtAddr<SkinLimb>>,
    pub limb_count: u8,
}

#[derive(Debug, FromBytes)]
#[repr(C)]
pub struct SkinLimb {
    pub joint_pos: [I16; 3],
    pub child: u8,
    pub sibling: u8,
    pub segment_type: I32,

    /// Gfx* if segmentType is SKIN_LIMB_TYPE_NORMAL,
    /// SkinAnimatedLimbData* if segmentType is SKIN_LIMB_TYPE_ANIMATED,
    /// NULL otherwise
    pub segment: RawVirtAddr,
}
const _: () = assert!(std::mem::size_of::<SkinLimb>() == 0x10);

#[derive(Debug, FromBytes)]
#[repr(C)]
pub struct SkinAnimatedLimbData {
    pub total_vtx_count: U16,
    pub limb_modif_count: U16,
    pub limb_modifications: VirtAddr<SkinLimbModif>,
    pub dlist: Gfx,
}
const _: () = assert!(std::mem::size_of::<SkinAnimatedLimbData>() == 0xC);

#[derive(Debug, AsBytes, FromBytes, Default, Clone)]
#[repr(C)]
pub struct Vtx {
    pub pos: [I16; 3],
    pub flag: I16,
    pub tpos: [I16; 2],
    pub cn: [u8; 4],
}

#[derive(FromBytes, Debug)]
#[repr(C)]
pub struct SkinLimbModif {
    pub vtx_count: U16,
    pub transform_count: U16,
    pub unk_4: Aligned4<U16>,
    pub skin_vertices: VirtAddr<SkinVertex>,
    pub limb_transformations: VirtAddr<SkinTransformation>,
}
const _: () = assert!(std::mem::size_of::<SkinLimbModif>() == 0x10);

#[derive(FromBytes)]
#[repr(C)]
pub struct SkinTransformation {
    pub limb_index: Aligned2<u8>,
    pub x: I16,
    pub y: I16,
    pub z: I16,
    pub scale: u8,
}
const _: () = assert!(std::mem::size_of::<SkinTransformation>() == 0xA);

#[derive(FromBytes)]
#[repr(C)]
pub struct SkinVertex {
    pub index: U16,
    pub s: I16,
    pub t: I16,
    pub norm_x: i8,
    pub norm_y: i8,
    pub norm_z: i8,
    pub alpha: u8,
}
const _: () = assert!(std::mem::size_of::<SkinVertex>() == 0xA);

#[derive(FromBytes, Debug)]
#[repr(C)]
pub struct AnimationHeaderCommon {
    pub frame_count: Aligned4<I16>,
}

#[derive(FromBytes, Debug)]
#[repr(C)]
pub struct AnimationHeader {
    pub common: AnimationHeaderCommon,
    pub frame_data: VirtAddr<I16>,
    pub joint_indicies: VirtAddr<JointIndex>,
    pub static_index_max: Aligned4<U16>,
}
const _: () = assert!(std::mem::size_of::<AnimationHeader>() == 0x10);

#[derive(FromBytes)]
#[repr(C)]
pub struct JointIndex {
    pub x: U16,
    pub y: U16,
    pub z: U16,
}
const _: () = assert!(std::mem::size_of::<JointIndex>() == 0x06);
