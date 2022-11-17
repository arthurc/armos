use std::{collections::HashMap, mem};

use ::gltf::json;
use ::gltf::json::validation::Checked::*;
use bytemuck::{Pod, Zeroable};

use crate::dlist::{for_each_instr, Opcode};

use super::*;

#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Vtx {
    pos: [i16; 3],
}

pub fn gltf_from_skeleton(header: &SkeletonHeader, r: &mut RomReader) -> Result<json::Root> {
    let mut meshes = Vec::new();
    let mut accessors = Vec::new();
    let mut buffers = Vec::new();
    let mut buffer_views = Vec::new();
    let mut nodes = Vec::new();

    for limb in &header.limbs {
        if let Some(animated_limb) = limb.animated_limb.as_ref() {
            let mut vtxs = vec![Vtx::default(); animated_limb.total_vtx_count as usize];

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
                    let mut v = &mut vtxs[skin_vertex.index as usize];
                    v.pos = [
                        vtx_point[0] as i16,
                        vtx_point[1] as i16,
                        vtx_point[2] as i16,
                    ];
                }
            }

            r.seek(animated_limb.dlist as _);
            // r.set_segment(Segment::IconItemStatic, )
            for_each_instr(r, |opcode, data| match opcode {
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
                }
                _ => (),
            })?;

            buffers.push(json::Buffer {
                byte_length: (vtxs.len() * mem::size_of::<Vtx>()) as _,
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                uri: Some(format!(
                    "data:application/octet-stream;base64,{}",
                    base64::encode(bytemuck::cast_slice(&vtxs))
                )),
            });

            buffer_views.push(json::buffer::View {
                buffer: json::Index::new(buffers.len() as _),
                byte_length: mem::size_of_val(&*vtxs) as _,
                byte_offset: None,
                byte_stride: Some(mem::size_of::<Vtx>() as _),
                extensions: Default::default(),
                extras: Default::default(),
                name: None,
                target: Some(Valid(json::buffer::Target::ArrayBuffer)),
            });

            accessors.push(json::Accessor {
                buffer_view: Some(json::Index::new(0)),
                byte_offset: 0,
                count: vtxs.len() as u32,
                component_type: Valid(json::accessor::GenericComponentType(
                    json::accessor::ComponentType::I16,
                )),
                extensions: Default::default(),
                extras: Default::default(),
                type_: Valid(json::accessor::Type::Vec3),
                min: Some(json::Value::from(Vec::from([0, 0, 0]))),
                max: Some(json::Value::from(Vec::from([0, 0, 0]))),
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
                    indices: None,
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
                mesh: Some(json::Index::new(meshes.len() as _)),
                name: None,
                rotation: None,
                scale: None,
                translation: None,
                skin: None,
                weights: None,
            });
        }
    }

    Ok(json::Root {
        buffers,
        buffer_views,
        accessors,
        meshes,
        nodes,
        ..Default::default()
    })
}
