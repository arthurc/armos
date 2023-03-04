use std::mem;

use glam::{Quat, Vec3};

use crate::n64::math::rotate_zyx;

use super::*;

#[derive(Debug)]
pub struct SkeletonHeader {
    pub limbs: Vec<SkinLimb>,
}
impl SkeletonHeader {
    pub fn read(r: &mut RomReader) -> Result<Self> {
        let segment = r.read_addr()?;
        let limb_count = r.read_u8()?;

        log::info!(
            "Reading skeleton header. segment: {}, limb_count: {}",
            segment,
            limb_count
        );

        Ok(Self {
            limbs: r
                .ptr_segment_iter(segment)
                .take(limb_count as usize)
                .collect::<Result<_>>()?,
        })
    }
}

#[derive(Debug)]
pub struct AnimationHeaderCommon {
    frame_count: i16,
}
impl ReadSegment for AnimationHeaderCommon {
    const SIZE: u32 = 0x02;

    fn read(r: &mut RomReader) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            frame_count: r.read_i16()?,
        })
    }
}

#[derive(Debug)]
pub struct AnimationHeader {
    common: AnimationHeaderCommon,
    frame_data: VirtualAddress,
    joint_indices: VirtualAddress,
    static_index_max: i16,
}
impl ReadSegment for AnimationHeader {
    const SIZE: u32 = 0x10;

    fn read(r: &mut RomReader) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let common = AnimationHeaderCommon::read(r)?;
        let _ = r.read_u16()?; // Padding
        let frame_data = r.read_addr()?;
        let joint_indices = r.read_addr()?;
        let static_index_max = r.read_i16()?;

        Ok(Self {
            common,
            frame_data,
            joint_indices,
            static_index_max,
        })
    }
}

#[derive(Debug)]
pub struct JointIndex {
    x: u16,
    y: u16,
    z: u16,
}
impl ReadSegment for JointIndex {
    const SIZE: u32 = 0x06;

    fn read(r: &mut RomReader) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            x: r.read_u16()?,
            y: r.read_u16()?,
            z: r.read_u16()?,
        })
    }
}

pub struct SkeletonAnimation {
    pub name: String,
    pub frames: Vec<Frame>,
}
impl SkeletonAnimation {
    pub fn create_from_header(
        name: String,
        r: &mut RomReader,
        skeleton: &SkeletonHeader,
        animation: &AnimationHeader,
    ) -> Result<Self> {
        let frame_data = |r: &mut RomReader, frame_index: i16, n: u16| {
            r.seek(
                animation.frame_data
                    + mem::size_of::<i16>() as i32 * (frame_index as i32 + n as i32),
            )
            .read_i16()
        };
        let static_index_max = animation.static_index_max;
        let joint_indices = r
            .segment_iter::<JointIndex>(animation.joint_indices.into())
            .take(skeleton.limbs.len() + 1)
            .collect::<Result<Vec<_>>>()?;

        log::info!(
            "Reading skeleton animation. static_index_max: {}  joint indices: {}  frame count: {}",
            static_index_max,
            joint_indices.len(),
            animation.common.frame_count
        );

        let mut frames = Vec::new();
        for frame_index in 0..animation.common.frame_count {
            log::trace!("Frame {}", frame_index);

            let limb_data = |r: &mut RomReader, n: u16| {
                if n as i16 >= static_index_max {
                    frame_data(r, frame_index, n)
                } else {
                    frame_data(r, 0, n)
                }
            };

            let mut joint_table = Vec::new();
            for limb_index in 0..(skeleton.limbs.len() + 1) {
                let joint_index = &joint_indices[limb_index];

                let x = limb_data(r, joint_index.x)?;
                let y = limb_data(r, joint_index.y)?;
                let z = limb_data(r, joint_index.z)?;

                log::trace!(
                    "  - joint_index: {}, {}, {}   pos: {}, {}, {}",
                    joint_index.x,
                    joint_index.y,
                    joint_index.z,
                    x,
                    y,
                    z
                );

                joint_table.push([x, y, z]);
            }

            let mut joints = Vec::new();
            for rot in joint_table.iter().skip(1) {
                joints.push(Joint {
                    rot: Vec3::from_array([rot[0] as _, rot[1] as _, rot[2] as _]),
                })
            }

            frames.push(Frame { joints });
        }

        Ok(Self { name, frames })
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub joints: Vec<Joint>,
}

#[derive(Debug, Clone)]
pub struct Joint {
    pub rot: Vec3,
}
impl Joint {
    pub fn rot_quat(&self) -> Quat {
        Quat::from_mat4(&rotate_zyx(
            self.rot[0] as _,
            self.rot[1] as _,
            self.rot[2] as _,
        ))
    }
}
