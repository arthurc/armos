use super::*;

use std::{collections::HashMap, mem};

use ::gltf::json;
use ::gltf::json::validation::Checked::*;
use bytemuck::{Pod, Zeroable};

use crate::{
    dlist::{InstrIter, Opcode},
    rom::{self, segment_offset},
};

struct Vtx {
    pos: [u16; 3],
    flag: i16,
    tpos: [i16; 2],
    cn: [u8; 4],
}

#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Vertex {
    pos: [f32; 3],
}

fn fold_pos(vtxs: &[Vertex], op: impl Fn(f32, f32) -> f32) -> Option<[f32; 3]> {
    vtxs.iter().fold(None, |acc, v| match acc {
        None => Some(v.pos.clone()),
        Some([x, y, z]) => Some([op(x, v.pos[0]), op(y, v.pos[1]), op(z, v.pos[2])]),
    })
}

pub fn gltf_from_skeleton(header: &SkeletonHeader, r: &mut RomReader) -> Result<json::Root> {
    let mut meshes = Vec::new();
    let mut accessors = Vec::new();
    let mut buffers = Vec::new();
    let mut buffer_views = Vec::new();
    let mut nodes = Vec::new();

    for limb in &header.limbs {
        if let Some(animated_limb) = limb.animated_limb.as_ref() {
            let mut vertex_buffer = vec![Vertex::default(); animated_limb.total_vtx_count as usize];

            // See Skin_ApplyLimbModifications
            for modif in &animated_limb.limb_modifications {
                let mut vtx_point = [0.0f32, 0.0f32, 0.0f32];
                for transformation in &modif.limb_transformations {
                    let scale = transformation.scale as f32 * 0.01f32;
                    let v = [
                        transformation.pos[0] as f32 * scale,
                        transformation.pos[1] as f32 * scale,
                        transformation.pos[2] as f32 * scale,
                    ];

                    vtx_point[0] += v[0];
                    vtx_point[1] += v[1];
                    vtx_point[2] += v[2];
                }

                // See Skin_UpdateVertices
                for skin_vertex in &modif.skin_vertices {
                    let mut v = &mut vertex_buffer[skin_vertex.index as usize];
                    v.pos = vtx_point;
                }
            }

            log::info!(
                "Vertex buffer constructed. len: {}, sizeof(Vtx): {}",
                vertex_buffer.len(),
                mem::size_of::<Vtx>()
            );

            r.set_segment(
                rom::Segment::IconItemStatic,
                Some(bytemuck::cast_slice(&vertex_buffer).to_vec()),
            );

            let mut vtx_offset = 0;
            let mut triangles = Vec::new();
            let mut iter = InstrIter::new(animated_limb.dlist);
            while let Some((opcode, data)) = iter.next(r)? {
                match opcode {
                    Opcode::VTX => {
                        let nn = ((data & 0x000FF00000000000u64) >> 44) as u32;
                        let aa = ((data & 0x000000FF00000000u64) >> 32) as u32;
                        let addr = data as u32;

                        log::trace!(
                            "VTX nn: {} aa: {} (aa >> 1) - nn: {} addr: 0x{:08X}",
                            nn,
                            aa,
                            (aa >> 1) - nn,
                            addr
                        );

                        vtx_offset = segment_offset(addr);
                    }
                    Opcode::TRI1 => {
                        // let aa = (((data & 0x0000000000FF0000u64) >> 16) / 2) as u32;
                        // let bb = (((data & 0x000000000000FF00u64) >> 8) / 2) as u32;
                        // let cc = (((data & 0x00000000000000FFu64) >> 0) / 2) as u32;

                        let aa = (((data & 0x00FF000000000000u64) >> 48) / 2) as u32;
                        let bb = (((data & 0x0000FF0000000000u64) >> 40) / 2) as u32;
                        let cc = (((data & 0x000000FF00000000u64) >> 32) / 2) as u32;

                        log::trace!("TRI1 aa: {} bb: {} cc: {}", aa, bb, cc,);

                        let start_index = vtx_offset / mem::size_of::<Vtx>() as u32;
                        triangles.push([start_index + aa, start_index + bb, start_index + cc]);
                    }
                    Opcode::TRI2 => {
                        let aa = (((data & 0x00FF000000000000u64) >> 48) / 2) as u32;
                        let bb = (((data & 0x0000FF0000000000u64) >> 40) / 2) as u32;
                        let cc = (((data & 0x000000FF00000000u64) >> 32) / 2) as u32;
                        let dd = (((data & 0x0000000000FF0000u64) >> 16) / 2) as u32;
                        let ee = (((data & 0x000000000000FF00u64) >> 8) / 2) as u32;
                        let ff = (((data & 0x00000000000000FFu64) >> 0) / 2) as u32;

                        log::trace!(
                            "TRI2 aa: {} bb: {} cc: {} dd: {} ee: {} ff: {}",
                            aa,
                            bb,
                            cc,
                            dd,
                            ee,
                            ff
                        );

                        let start_index = vtx_offset / mem::size_of::<Vtx>() as u32;
                        triangles.push([start_index + aa, start_index + bb, start_index + cc]);
                        triangles.push([start_index + dd, start_index + ee, start_index + ff]);
                    }
                    _ => (),
                }
            }

            let min = fold_pos(&vertex_buffer, |a, b| a.min(b));
            let max = fold_pos(&vertex_buffer, |a, b| a.max(b));

            log::info!(
                "Triangles buffer constructed. len: {}, min: {:?}, max: {:?}",
                triangles.len(),
                min,
                max
            );

            buffers.push(json::Buffer {
                byte_length: mem::size_of_val(&*vertex_buffer) as _,
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                uri: Some(format!(
                    "data:application/octet-stream;base64,{}",
                    base64::encode(bytemuck::cast_slice(&vertex_buffer))
                )),
            });
            buffer_views.push(json::buffer::View {
                buffer: json::Index::new(buffers.len() as u32 - 1),
                byte_length: mem::size_of_val(&*vertex_buffer) as _,
                byte_offset: None,
                byte_stride: Some(mem::size_of::<Vertex>() as _),
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                target: Some(Valid(json::buffer::Target::ArrayBuffer)),
            });
            accessors.push(json::Accessor {
                buffer_view: Some(json::Index::new(buffer_views.len() as u32 - 1)),
                byte_offset: 0,
                count: vertex_buffer.len() as u32,
                component_type: Valid(json::accessor::GenericComponentType(
                    json::accessor::ComponentType::F32,
                )),
                extensions: Default::default(),
                extras: Default::default(),
                type_: Valid(json::accessor::Type::Vec3),
                min: min.map(|v| json::Value::from(v.to_vec())),
                max: max.map(|v| json::Value::from(v.to_vec())),
                name: None,
                normalized: false,
                sparse: None,
            });

            buffers.push(json::Buffer {
                byte_length: mem::size_of_val(&*triangles) as _,
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                uri: Some(format!(
                    "data:application/octet-stream;base64,{}",
                    base64::encode(bytemuck::cast_slice(&triangles))
                )),
            });
            buffer_views.push(json::buffer::View {
                buffer: json::Index::new(buffers.len() as u32 - 1),
                byte_length: mem::size_of_val(&*triangles) as u32,
                byte_offset: None,
                byte_stride: None,
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                target: Some(Valid(json::buffer::Target::ElementArrayBuffer)),
            });
            accessors.push(json::Accessor {
                buffer_view: Some(json::Index::new(buffer_views.len() as u32 - 1)),
                byte_offset: 0,
                count: triangles.len() as u32 * 3,
                component_type: Valid(json::accessor::GenericComponentType(
                    json::accessor::ComponentType::U32,
                )),
                extensions: Default::default(),
                extras: Default::default(),
                type_: Valid(json::accessor::Type::Scalar),
                min: None,
                max: None,
                name: None,
                normalized: false,
                sparse: None,
            });

            meshes.push(json::Mesh {
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                primitives: vec![json::mesh::Primitive {
                    attributes: {
                        let mut map = HashMap::new();
                        map.insert(Valid(json::mesh::Semantic::Positions), json::Index::new(0));
                        map
                    },
                    extensions: Default::default(),
                    extras: Default::default(),
                    indices: Some(json::Index::new(accessors.len() as u32 - 1)),
                    material: None,
                    mode: Valid(json::mesh::Mode::Triangles),
                    targets: None,
                }],
                weights: None,
            });

            nodes.push(json::Node {
                camera: None,
                children: None,
                extensions: Default::default(),
                extras: Default::default(),
                matrix: None,
                mesh: Some(json::Index::new(meshes.len() as u32 - 1)),
                name: None,
                rotation: None,
                scale: None,
                translation: None,
                skin: None,
                weights: None,
            });
        }
    }

    let scenes = vec![json::Scene {
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        nodes: vec![json::Index::new(0)],
    }];

    Ok(json::Root {
        buffers,
        buffer_views,
        accessors,
        meshes,
        nodes,
        scenes,
        ..Default::default()
    })
}
