use std::{
    fmt::Debug,
    io::{self, Read, Seek},
};

use crate::rom::{ReadSegment, RomReader};
use anyhow::Result;

#[derive(Debug)]
pub struct SkeletonHeader {
    limbs: Vec<SkinLimb>,
}
impl SkeletonHeader {
    pub fn read(r: &mut RomReader<impl Read + Seek>) -> Result<Self> {
        let segment = r.read_u32()?;
        let limb_count = r.read_u8()?;

        log::info!("Reading segment 0x{:08X}, count: {}", segment, limb_count);

        Ok(Self {
            limbs: r
                .ptr_segment_iter(segment)
                .take(limb_count as usize)
                .collect::<io::Result<_>>()?,
        })
    }
}

#[derive(Default)]
pub struct SkinLimb {
    joint_pos: [i16; 3],
    child: u8,
    sibling: u8,
    segment_type: i32,
    animated_limb: Option<SkinAnimatedLimbData>,
}
impl<R> ReadSegment<R> for SkinLimb
where
    R: Read + Seek,
{
    const SIZE: usize = 0x10;

    fn read(r: &mut RomReader<R>) -> io::Result<Self> {
        let joint_pos = [r.read_i16()?, r.read_i16()?, r.read_i16()?];
        let child = r.read_u8()?;
        let sibling = r.read_u8()?;
        let segment_type = r.read_i32()?;
        let segment = r.read_u32()?;
        let animated_limb = if segment_type == 4 && segment != 0 {
            r.seek(segment as _)?;
            Some(r.read_segment::<SkinAnimatedLimbData>()?)
        } else {
            None
        };

        Ok(Self {
            joint_pos,
            child,
            sibling,
            segment_type,
            animated_limb,
        })
    }
}
impl Debug for SkinLimb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkinLimb")
            .field("joint_pos", &self.joint_pos)
            .field("child", &self.child)
            .field("sibling", &self.sibling)
            .field("segment_type", &self.segment_type)
            .field("animated_limb", &self.animated_limb)
            .finish()
    }
}

#[derive(Debug)]
pub struct SkinAnimatedLimbData {
    total_vtx_count: u16,
    limb_modifications: Vec<SkinLimbModif>,
}
impl<R> ReadSegment<R> for SkinAnimatedLimbData
where
    R: Read + Seek,
{
    const SIZE: usize = 0xC;

    fn read(r: &mut RomReader<R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        let total_vtx_count = r.read_u16()?;
        let limb_modif_count = r.read_u16()?;
        let limb_modifications = r.read_u32()?;

        Ok(Self {
            total_vtx_count,
            limb_modifications: r
                .segment_iter(limb_modifications)
                .take(limb_modif_count as _)
                .collect::<io::Result<_>>()?,
        })
    }
}

pub struct SkinLimbModif {
    vtx_count: u16,
    transform_count: u16,
    unk_4: u16,
    skin_vertices: Vec<SkinVertex>,
    limb_transformations: u32,
}
impl<R> ReadSegment<R> for SkinLimbModif
where
    R: Read + Seek,
{
    const SIZE: usize = 0x10;

    fn read(r: &mut RomReader<R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        let vtx_count = r.read_u16()?;
        let transform_count = r.read_u16()?;
        let unk_4 = r.read_u16()?;
        let _ = r.read_u16()?;
        let skin_vertices = r.read_u32()?;
        let limb_transformations = r.read_u32()?;

        Ok(Self {
            vtx_count,
            transform_count,
            unk_4,
            skin_vertices: r
                .segment_iter(skin_vertices)
                .take(vtx_count as _)
                .collect::<io::Result<_>>()?,
            limb_transformations,
        })
    }
}
impl Debug for SkinLimbModif {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkinLimbModif")
            .field("vtx_count", &self.vtx_count)
            .field("transform_count", &self.transform_count)
            .field("unk_4", &self.unk_4)
            .field("skin_vertices", &self.skin_vertices)
            .field(
                "limb_transformations",
                &format_args!("0x{:08X}", self.limb_transformations),
            )
            .finish()
    }
}

#[derive(Debug)]
struct SkinVertex {
    index: u16,
    pos: [i16; 3],
    scale: u8,
}
impl<R> ReadSegment<R> for SkinVertex
where
    R: Read,
{
    const SIZE: usize = 0xA;

    fn read(r: &mut RomReader<R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            index: r.read_u16()?,
            pos: [r.read_i16()?, r.read_i16()?, r.read_i16()?],
            scale: r.read_u8()?,
        })
    }
}
