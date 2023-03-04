pub mod gltf;
mod skeleton;

use crate::rom::{ReadSegment, RomReader, VirtualAddress};
use anyhow::Result;
use std::fmt::Debug;

use self::skeleton::{AnimationHeader, SkeletonAnimation, SkeletonHeader};

pub struct Skin {
    skeleton_header: SkeletonHeader,
    animations: Vec<SkeletonAnimation>,
}
impl Skin {
    pub fn read(r: &mut RomReader, addr: VirtualAddress) -> Result<Self> {
        Ok(Self {
            skeleton_header: SkeletonHeader::read(r.seek(addr))?,
            animations: Vec::new(),
        })
    }

    pub fn read_animation(
        &mut self,
        name: impl Into<String>,
        r: &mut RomReader,
        addr: VirtualAddress,
    ) -> Result<()> {
        let name = name.into();
        log::info!("Reading {}", name);

        let animation_header = AnimationHeader::read(r.seek(addr))?;
        self.animations.push(SkeletonAnimation::create_from_header(
            name,
            r,
            &self.skeleton_header,
            &animation_header,
        )?);

        Ok(())
    }

    pub fn to_gltf(&self, r: &mut RomReader) -> Result<::gltf::json::Root> {
        log::info!("Creating skin gltf");

        let mut root = gltf::gltf_from_skeleton(&self.skeleton_header, r)?;

        for animation in &self.animations {
            gltf::add_animation(
                &animation.name,
                &mut root,
                &self.skeleton_header,
                &animation,
            )?;
        }

        Ok(root)
    }
}

#[derive(Debug)]
enum SkinLimbType {
    Animated(SkinAnimatedLimbData),
    Normal(VirtualAddress),
}

#[derive(Debug, Default)]
pub struct SkinLimb {
    joint_pos: [i16; 3],
    child: u8,
    sibling: u8,
    segment_type: i32,
    skin_limb_type: Option<SkinLimbType>,
}
impl ReadSegment for SkinLimb {
    const SIZE: u32 = 0x10;

    fn read(r: &mut RomReader) -> Result<Self> {
        let joint_pos = [r.read_i16()?, r.read_i16()?, r.read_i16()?];
        let child = r.read_u8()?;
        let sibling = r.read_u8()?;
        let segment_type = r.read_i32()?;
        let segment = r.read_addr()?;

        log::info!(
            "Skin limb segment @ {}, segment_type: {:>2}, child: {:>3}, sibling: {:>3}",
            segment,
            segment_type,
            child,
            sibling
        );

        let skin_limb_type = if segment_type == 4 && segment != VirtualAddress::NULL {
            log::info!("  - Animated type @ {}", segment);
            Some(SkinLimbType::Animated(
                r.seek(segment).read_segment::<SkinAnimatedLimbData>()?,
            ))
        } else if segment_type == 11 && segment != VirtualAddress::NULL {
            log::info!("  - Normal type @ {}", segment);
            Some(SkinLimbType::Normal(segment))
        } else {
            None
        };

        Ok(Self {
            joint_pos,
            child,
            sibling,
            segment_type,
            skin_limb_type,
        })
    }
}

#[derive(Debug)]
pub struct SkinAnimatedLimbData {
    total_vtx_count: u16,
    limb_modifications: Vec<SkinLimbModif>,
    dlist: VirtualAddress,
}
impl ReadSegment for SkinAnimatedLimbData {
    const SIZE: u32 = 0xC;

    fn read(r: &mut RomReader) -> Result<Self>
    where
        Self: Sized,
    {
        let total_vtx_count = r.read_u16()?;
        let limb_modif_count = r.read_u16()?;
        let limb_modifications = r.read_addr()?;
        let dlist = r.read_addr()?;

        log::info!(
            "Reading skin animated limb data. limb_modif_count: {}",
            limb_modif_count
        );

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
        let skin_vertices = r.read_addr()?;
        let limb_transformations = r.read_addr()?;

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
