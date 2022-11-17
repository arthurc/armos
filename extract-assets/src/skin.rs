pub mod gltf;

use crate::rom::{ReadSegment, RomReader};
use anyhow::Result;
use std::fmt::Debug;

#[derive(Debug)]
pub struct SkeletonHeader {
    limbs: Vec<SkinLimb>,
}
impl SkeletonHeader {
    pub fn read(r: &mut RomReader) -> Result<Self> {
        let segment = r.read_u32()?;
        let limb_count = r.read_u8()?;

        log::info!("Reading segment 0x{:08X}, count: {}", segment, limb_count);

        Ok(Self {
            limbs: r
                .ptr_segment_iter(segment)
                .take(limb_count as usize)
                .collect::<Result<_>>()?,
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
impl ReadSegment for SkinLimb {
    const SIZE: u32 = 0x10;

    fn read(r: &mut RomReader) -> Result<Self> {
        let joint_pos = [r.read_i16()?, r.read_i16()?, r.read_i16()?];
        let child = r.read_u8()?;
        let sibling = r.read_u8()?;
        let segment_type = r.read_i32()?;
        let segment = r.read_u32()?;
        let animated_limb = if segment_type == 4 && segment != 0 {
            r.seek(segment as _);
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

pub struct SkinAnimatedLimbData {
    total_vtx_count: u16,
    limb_modifications: Vec<SkinLimbModif>,
    dlist: u32,
}
impl ReadSegment for SkinAnimatedLimbData {
    const SIZE: u32 = 0xC;

    fn read(r: &mut RomReader) -> Result<Self>
    where
        Self: Sized,
    {
        let total_vtx_count = r.read_u16()?;
        let limb_modif_count = r.read_u16()?;
        let limb_modifications = r.read_u32()?;
        let dlist = r.read_u32()?;

        Ok(Self {
            total_vtx_count,
            limb_modifications: r
                .segment_iter(limb_modifications)
                .take(limb_modif_count as _)
                .collect::<Result<_>>()?,
            dlist,
        })
    }
}
impl Debug for SkinAnimatedLimbData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkinAnimatedLimbData")
            .field("total_vtx_count", &self.total_vtx_count)
            .field("limb_modifications", &self.limb_modifications)
            .field("dlist", &format_args!("0x{:08X}", self.dlist))
            .finish()
    }
}

#[derive(Debug)]
pub struct SkinLimbModif {
    unk_4: u16,
    skin_vertices: Vec<SkinVertex>,
    limb_transformations: Vec<SkinTransformation>,
}
impl ReadSegment for SkinLimbModif {
    const SIZE: u32 = 0x10;

    fn read(r: &mut RomReader) -> Result<Self>
    where
        Self: Sized,
    {
        let vtx_count = r.read_u16()?;
        let transform_count = r.read_u16()?;
        let unk_4 = r.read_u16()?;
        let _ = r.read_u16()?; // Padding
        let skin_vertices = r.read_u32()?;
        let limb_transformations = r.read_u32()?;

        Ok(Self {
            unk_4,
            skin_vertices: r
                .segment_iter(skin_vertices)
                .take(vtx_count as _)
                .collect::<Result<_>>()?,
            limb_transformations: r
                .segment_iter(limb_transformations)
                .take(transform_count as _)
                .collect::<Result<_>>()?,
        })
    }
}

#[derive(Debug)]
struct SkinVertex {
    index: u16,
    s: i16,
    t: i16,
    norm: [i8; 3],
    alpha: u8,
}
impl ReadSegment for SkinVertex {
    const SIZE: u32 = 0xA;

    fn read(r: &mut RomReader) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            index: r.read_u16()?,
            s: r.read_i16()?,
            t: r.read_i16()?,
            norm: [r.read_i8()?, r.read_i8()?, r.read_i8()?],
            alpha: r.read_u8()?,
        })
    }
}

#[derive(Debug)]
struct SkinTransformation {
    limb_index: u8,
    pos: [i16; 3],
    scale: u8,
}
impl ReadSegment for SkinTransformation {
    const SIZE: u32 = 0xA;

    fn read(r: &mut RomReader) -> Result<Self>
    where
        Self: Sized,
    {
        let limb_index = r.read_u8()?;
        let _ = r.read_u8()?; // Padding
        let pos = [r.read_i16()?, r.read_i16()?, r.read_i16()?];
        let scale = r.read_u8()?;

        Ok(Self {
            limb_index,
            pos,
            scale,
        })
    }
}
