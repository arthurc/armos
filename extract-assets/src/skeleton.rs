use anyhow::{Context, Result};
use gltf::json::{self, Index};
use log::Level;
use num_traits::FromPrimitive;
use zerocopy::AsBytes;

use crate::{
    addr::VirtAddr,
    display_list::{self, InstructionStream},
    mesh, rom, skeleton_animation,
};

pub fn read_into_gltf(
    root: &mut json::Root,
    reader: &rom::Reader,
    addr: VirtAddr<rom::SkeletonHeader>,
    animation_addrs: &[VirtAddr<rom::AnimationHeader>],
) -> Result<()> {
    let skeleton_header = reader
        .read(addr)
        .context("Failed to read skeleton header")?;
    let limbs = reader
        .ptr_slice_iter(skeleton_header.limbs, skeleton_header.limb_count as usize)
        .context("Failed to read limbs")?
        .collect::<Vec<_>>();

    log::info!("Creating skeleton skin nodes");
    for limb in &limbs {
        let mesh = match FromPrimitive::from_i32(limb.segment_type.get()) {
            Some(rom::SkinLimbType::Normal) => {
                log::info!("  Normal skin limb, segment:{}", limb.segment);
                Some(read_normal_skin_limb(reader, &limb)?)
            }
            Some(rom::SkinLimbType::Animated) => {
                log::info!("  Animated skin limb, segment:{}", limb.segment);
                Some(read_animated_skin_limb(reader, &limb)?)
            }
            _ => None,
        };

        if let Some(mesh) = mesh.as_ref() {
            mesh.write_into_gltf(root);
        }

        root.nodes.push(json::Node {
            camera: None,
            children: None,
            extensions: Default::default(),
            extras: Default::default(),
            matrix: None,
            mesh: mesh.map(|_| Index::new(root.meshes.len() as u32 - 1)),
            name: None,
            rotation: None,
            scale: None,
            translation: Some([
                limb.joint_pos[0].get() as _,
                limb.joint_pos[1].get() as _,
                limb.joint_pos[2].get() as _,
            ]),
            skin: None,
            weights: None,
        });
    }

    log::info!("Building skeleton node hierarchy");
    build_node_hierarchy(root, &limbs);

    if log::log_enabled!(Level::Trace) {
        for (index, node) in root.nodes.iter().enumerate() {
            log::trace!(
                "  Node {} as {} children",
                index,
                node.children
                    .as_ref()
                    .map(|n| format!("{}", n.len()))
                    .unwrap_or("null".to_owned())
            );
        }
    }

    for animation_addr in animation_addrs {
        skeleton_animation::read_into_gltf(root, reader, &skeleton_header, *animation_addr)?;
    }

    Ok(())
}

fn read_normal_skin_limb(reader: &rom::Reader, limb: &rom::SkinLimb) -> Result<mesh::Mesh> {
    let mut instruction_stream = InstructionStream::new(
        reader
            .slice_from(limb.segment)
            .with_context(|| format!("Could not read data for at address {}", limb.segment))?,
    );

    if log::log_enabled!(Level::Trace) {
        log::trace!("Display list instructions:");
        instruction_stream.clone().for_each(display_list::dump());
    }

    instruction_stream.try_fold(mesh::Mesh::default(), mesh::fold(reader))
}

fn read_animated_skin_limb(reader: &rom::Reader, limb: &rom::SkinLimb) -> Result<mesh::Mesh> {
    let rom::SkinAnimatedLimbData {
        limb_modifications,
        limb_modif_count,
        total_vtx_count,
        dlist,
    } = reader
        .read::<rom::SkinAnimatedLimbData>(limb.segment.into())
        .context("Failed to read skin animated limb data")?;

    let limb_modifs = reader
        .read_slice(limb_modifications, limb_modif_count.get() as _)
        .context("Failed to read skin limb modifications")?;

    let mut vtx_buffer = vec![rom::Vtx::default(); total_vtx_count.get() as _];
    for modif in limb_modifs {
        let limb_transformations = reader
            .read_slice(modif.limb_transformations, modif.transform_count.get() as _)
            .context("Failed to read limb transformations")?;
        let skin_vertices = reader
            .read_slice(modif.skin_vertices, modif.vtx_count.get() as _)
            .context("Failed to read skin vertices")?;

        let vtx_point = apply_limb_transformations(&limb_transformations);

        for skin_vertex in skin_vertices {
            vtx_buffer[skin_vertex.index.get() as usize].pos = [
                (vtx_point[0] as i16).into(),
                (vtx_point[1] as i16).into(),
                (vtx_point[2] as i16).into(),
            ];
        }
    }

    let mut reader = reader.clone();
    reader.set_segment(
        rom::Segment::IconItemStatic,
        Some(vtx_buffer.as_bytes().to_vec()),
    );

    let mut instruction_stream = display_list::InstructionStream::new(
        reader
            .slice_from(dlist)
            .context("Could not read animated skin limb display list")?,
    );

    if log::log_enabled!(Level::Trace) {
        log::trace!("Animated skin limb display list");
        instruction_stream.clone().for_each(display_list::dump());
    }

    instruction_stream.try_fold(mesh::Mesh::default(), mesh::fold(&reader))
}

fn apply_limb_transformations(limb_transformations: &[rom::SkinTransformation]) -> [f32; 3] {
    limb_transformations.iter().fold(
        Default::default(),
        |accum, rom::SkinTransformation { scale, x, y, z, .. }| {
            let scale = *scale as f32 * 0.01;

            [
                accum[0] + x.get() as f32 * scale,
                accum[1] + y.get() as f32 * scale,
                accum[2] + z.get() as f32 * scale,
            ]
        },
    )
}

fn build_node_hierarchy(root: &mut json::Root, limbs: &[rom::SkinLimb]) {
    for (index, rom::SkinLimb { child, .. }) in limbs
        .iter()
        .enumerate()
        .filter(|(_, limb)| limb.child != 0xFF)
    {
        let children = root.nodes[index].children.get_or_insert_with(Vec::new);

        children.push(Index::new(*child as _));

        let mut child_sibling = limbs[*child as usize].sibling;
        while child_sibling != 0xFF {
            children.push(Index::new(child_sibling as _));
            child_sibling = limbs[child_sibling as usize].sibling;
        }
    }
}
