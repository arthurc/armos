use std::mem;

use ::gltf::json::{self, validation::Checked::*};
use anyhow::Result;

use crate::skin::SkeletonHeader;

use super::SkeletonAnimation;

pub fn add_animation(
    name: &str,
    root: &mut json::Root,
    skeleton_header: &SkeletonHeader,
    animation: &SkeletonAnimation,
) -> Result<()> {
    let times: Vec<f32> = animation
        .frames
        .iter()
        .enumerate()
        .map(|(i, _)| i as f32 * 0.1)
        .collect();

    root.buffers.push(json::Buffer {
        byte_length: mem::size_of_val(&*times) as _,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: Some(format!(
            "data:application/octet-stream;base64,{}",
            base64::encode(bytemuck::cast_slice(&times))
        )),
    });
    root.buffer_views.push(json::buffer::View {
        buffer: json::Index::new(root.buffers.len() as u32 - 1),
        byte_length: mem::size_of_val(&*times) as _,
        byte_offset: None,
        byte_stride: None,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: None,
    });

    root.accessors.push(json::Accessor {
        buffer_view: Some(json::Index::new(root.buffer_views.len() as u32 - 1)),
        byte_offset: 0,
        count: times.len() as u32,
        component_type: Valid(json::accessor::GenericComponentType(
            json::accessor::ComponentType::F32,
        )),
        extensions: Default::default(),
        extras: Default::default(),
        type_: Valid(json::accessor::Type::Scalar),
        min: times
            .iter()
            .fold(None, |a, b| match a {
                None => Some(*b),
                Some(a) => Some(a.min(*b)),
            })
            .map(|n| json::Value::from(vec![n])),
        max: times
            .iter()
            .fold(None, |a, b| match a {
                None => Some(*b),
                Some(a) => Some(a.max(*b)),
            })
            .map(|n| json::Value::from(vec![n])),
        name: None,
        normalized: false,
        sparse: None,
    });
    let times_accessor_index = root.accessors.len() - 1;

    root.buffers.push(json::Buffer {
        byte_length: 0,
        extensions: Default::default(),
        extras: Default::default(),
        name: Some(String::from("rotations")),
        uri: None,
    });
    let rotations_buffer_index = root.buffers.len() - 1;

    root.buffer_views.push(json::buffer::View {
        buffer: json::Index::new(rotations_buffer_index as _),
        byte_length: 0,
        byte_offset: None,
        byte_stride: None,
        extensions: Default::default(),
        extras: Default::default(),
        name: Some(String::from("rotations")),
        target: None,
    });
    let rotations_buffer_view_index = root.buffer_views.len() - 1;

    let mut rotations = Vec::new();
    let mut samplers = Vec::new();
    let mut channels = Vec::new();
    for (limb_index, _) in skeleton_header.limbs.iter().enumerate() {
        root.accessors.push(json::Accessor {
            buffer_view: Some(json::Index::new(rotations_buffer_view_index as _)),
            byte_offset: mem::size_of_val(&*rotations) as _,
            count: animation.frames.len() as u32,
            component_type: Valid(json::accessor::GenericComponentType(
                json::accessor::ComponentType::F32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(json::accessor::Type::Vec4),
            min: None,
            max: None,
            name: Some(String::from("rotations")),
            normalized: false,
            sparse: None,
        });
        let rotations_accessor_index = root.accessors.len() - 1;

        samplers.push(json::animation::Sampler {
            input: json::Index::new(times_accessor_index as u32),
            interpolation: Valid(json::animation::Interpolation::Linear),
            output: json::Index::new(rotations_accessor_index as u32),
            extensions: Default::default(),
            extras: Default::default(),
        });
        channels.push(json::animation::Channel {
            sampler: json::Index::new(samplers.len() as u32 - 1),
            target: json::animation::Target {
                node: json::Index::new(limb_index as _),
                path: Valid(json::animation::Property::Rotation),
                extensions: Default::default(),
                extras: Default::default(),
            },
            extensions: Default::default(),
            extras: Default::default(),
        });

        for frame in &animation.frames {
            rotations.push(frame.joints[limb_index].rotation);
        }
    }

    root.buffers[rotations_buffer_index].byte_length = mem::size_of_val(&*rotations) as _;
    root.buffers[rotations_buffer_index].uri = Some(format!(
        "data:application/octet-stream;base64,{}",
        base64::encode(bytemuck::cast_slice(&rotations))
    ));
    root.buffer_views[rotations_buffer_view_index].byte_length =
        root.buffers[rotations_buffer_index].byte_length;

    root.animations.push(json::animation::Animation {
        samplers,
        channels,
        extensions: Default::default(),
        extras: Default::default(),
        name: Some(String::from(name)),
    });

    Ok(())
}
