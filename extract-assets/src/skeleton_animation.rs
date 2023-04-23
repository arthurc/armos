use std::mem;

use anyhow::{Context, Result};
use base64::prelude::*;
use glam::Quat;
use gltf::json::{self, validation::Checked::Valid};
use zerocopy::AsBytes;

use crate::{addr::VirtAddr, math, rom};

pub fn read_into_gltf(
    root: &mut json::Root,
    reader: &crate::rom::Reader,
    skeleton_header: &rom::SkeletonHeader,
    addr: VirtAddr<rom::AnimationHeader>,
) -> Result<()> {
    log::info!("Reading skeleton animation");

    let animation_header = reader
        .read(addr)
        .context("Failed to read animation header")?;

    log::info!("Adding times buffer");
    write_times_buffer_to_gltf(root, animation_header.common.frame_count.get() as _);

    log::info!("Adding animation frame buffers");
    write_animation_frames_to_gltf(root, reader, &animation_header, skeleton_header)?;

    Ok(())
}

fn for_each_frame_data<F>(
    reader: &rom::Reader,
    animation_header: &rom::AnimationHeader,
    frame_index: usize,
    limb_count: usize,
    mut f: F,
) -> Result<()>
where
    F: FnMut(usize, i16, i16, i16) -> (),
{
    let static_index_max = animation_header.static_index_max.get();

    let joint_indicies = reader
        .read_slice(animation_header.joint_indicies, limb_count as usize + 1)
        .context("Failed to read joint indicies")?;
    let frame_data = |n: i16| {
        reader
            .read(animation_header.frame_data + n as i32)
            .map(|n| n.get())
    };
    let static_data = |n: u16| frame_data(n as i16);
    let dynamic_data = |n: u16| frame_data(frame_index as i16 + n as i16);
    let read_data = |n: u16| {
        if n >= static_index_max {
            dynamic_data(n)
        } else {
            static_data(n)
        }
    };

    for limb_index in 0..limb_count {
        let joint_index = &joint_indicies[limb_index as usize + 1];
        let x = read_data(joint_index.x.get())?;
        let y = read_data(joint_index.y.get())?;
        let z = read_data(joint_index.z.get())?;

        log::trace!(
            "  - Frame [{: >3}]  Joint [{: >3}, {: >3}, {: >3}]  Pos [{: >6}, {: >6}, {: >6}]",
            frame_index,
            joint_index.x.get(),
            joint_index.y.get(),
            joint_index.z.get(),
            x,
            y,
            z,
        );

        f(limb_index, x, y, z);
    }

    Ok(())
}

fn write_times_buffer_to_gltf(root: &mut json::Root, frame_count: usize) {
    let times = (0..frame_count)
        .enumerate()
        .map(|(i, _)| i as f32 * 0.1)
        .collect::<Vec<_>>();

    root.buffers.push(json::Buffer {
        byte_length: mem::size_of_val(&*times) as _,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: Some(format!(
            "data:application/octet-stream;base64,{}",
            BASE64_STANDARD.encode(times.as_bytes())
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
        min: times.first().map(|n| json::Value::from(vec![*n])),
        max: times.last().map(|n| json::Value::from(vec![*n])),
        name: None,
        normalized: false,
        sparse: None,
    });
}

fn write_animation_frames_to_gltf(
    root: &mut json::Root,
    reader: &rom::Reader,
    animation_header: &rom::AnimationHeader,
    skeleton_header: &rom::SkeletonHeader,
) -> Result<()> {
    let times_accessor_index = root.accessors.len() as u32 - 1;

    let mut frame_table = vec![Vec::<[f32; 4]>::new(); skeleton_header.limb_count as usize + 1];
    for frame_index in 0..animation_header.common.frame_count.get() {
        for_each_frame_data(
            reader,
            &animation_header,
            frame_index as _,
            skeleton_header.limb_count as _,
            |limb_index, x, y, z| {
                // let q = Quat::from_euler(EulerRot::ZYX, x as _, y as _, z as _);
                // dbg!(x, y, z, q);

                //frame_table[limb_index].push(
                //    Quaternion::from(Euler::new(Rad(x as f32), Rad(y as f32), Rad(z as f32)))
                //        .into(),
                //)

                //frame_table[limb_index]
                //    .push(Quat::from_euler(EulerRot::XYZ, x as _, y as _, z as _).to_array())

                //let eul = EulerAngles::<_, IntraZYX>::from([x as f32, y as f32, z as f32]);
                // let x = Quaternion::from(eul);

                //dbg!(x, y, z);
                //dbg!(Quat::from_mat4(&math::rotate_zyx(x, y, z)).to_array());

                frame_table[limb_index].push(Quat::from_mat4(&math::rotate_zyx(x, y, z)).to_array())
            },
        )?;
    }

    let mut animation = json::animation::Animation {
        samplers: Default::default(),
        channels: Default::default(),
        extensions: Default::default(),
        extras: Default::default(),
        name: Some(String::from("anim")),
    };
    for limb_index in 0..skeleton_header.limb_count {
        let sampler_index = animation.samplers.len() as u32;
        let buffer_view_index = root.buffer_views.len() as u32;
        let buffer_index = root.buffers.len() as u32;
        let accessor_index = root.accessors.len() as u32;

        let bytes = frame_table[limb_index as usize].as_bytes();

        root.buffers.push(json::Buffer {
            byte_length: bytes.len() as u32,
            extensions: Default::default(),
            extras: Default::default(),
            name: Some(String::from("rotations")),
            uri: Some(format!(
                "data:application/octet-stream;base64,{}",
                BASE64_STANDARD.encode(bytes)
            )),
        });

        root.buffer_views.push(json::buffer::View {
            buffer: json::Index::new(buffer_index),
            byte_length: bytes.len() as u32,
            byte_offset: None,
            byte_stride: None,
            extensions: Default::default(),
            extras: Default::default(),
            name: Some(String::from("rotations")),
            target: None,
        });

        root.accessors.push(json::Accessor {
            buffer_view: Some(json::Index::new(buffer_view_index)),
            byte_offset: 0,
            count: animation_header.common.frame_count.get() as _,
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

        animation.channels.push(json::animation::Channel {
            sampler: json::Index::new(sampler_index),
            target: json::animation::Target {
                node: json::Index::new(limb_index as _),
                path: Valid(json::animation::Property::Rotation),
                extensions: Default::default(),
                extras: Default::default(),
            },
            extensions: Default::default(),
            extras: Default::default(),
        });

        animation.samplers.push(json::animation::Sampler {
            input: json::Index::new(times_accessor_index),
            interpolation: Valid(json::animation::Interpolation::Linear),
            output: json::Index::new(accessor_index),
            extensions: Default::default(),
            extras: Default::default(),
        });
    }

    root.animations.push(animation);

    Ok(())
}
