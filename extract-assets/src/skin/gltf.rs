use super::*;

use std::{collections::HashMap, mem};

use ::gltf::json::{self, validation::Checked::*};

use crate::{
    dlist::{
        self,
        gltf::{fold_pos, DisplayListData, Vertex},
        Vtx,
    },
    rom,
};

fn create_mesh(root: &mut json::Root, vertices: Vec<Vertex>, indices: Vec<u32>) {
    let min = fold_pos(&vertices, |a, b| a.min(b));
    let max = fold_pos(&vertices, |a, b| a.max(b));

    log::info!(
        "Creating skin mesh with {} vertices and {} indices",
        vertices.len(),
        indices.len()
    );

    for v in &vertices {
        println!("vtxPoint x: {}, y: {}, z: {}", v.pos[0], v.pos[1], v.pos[2]);
    }

    root.buffers.push(json::Buffer {
        byte_length: mem::size_of_val(&*vertices) as _,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: Some(format!(
            "data:application/octet-stream;base64,{}",
            base64::encode(bytemuck::cast_slice(&vertices))
        )),
    });
    root.buffer_views.push(json::buffer::View {
        buffer: json::Index::new(root.buffers.len() as u32 - 1),
        byte_length: mem::size_of_val(&*vertices) as _,
        byte_offset: None,
        byte_stride: Some(mem::size_of::<Vertex>() as _),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Valid(json::buffer::Target::ArrayBuffer)),
    });
    root.accessors.push(json::Accessor {
        buffer_view: Some(json::Index::new(root.buffer_views.len() as u32 - 1)),
        byte_offset: 0,
        count: vertices.len() as u32,
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

    root.buffers.push(json::Buffer {
        byte_length: mem::size_of_val(&*indices) as _,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: Some(format!(
            "data:application/octet-stream;base64,{}",
            base64::encode(bytemuck::cast_slice(&indices))
        )),
    });
    root.buffer_views.push(json::buffer::View {
        buffer: json::Index::new(root.buffers.len() as u32 - 1),
        byte_length: mem::size_of_val(&*indices) as u32,
        byte_offset: None,
        byte_stride: None,
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        target: Some(Valid(json::buffer::Target::ElementArrayBuffer)),
    });
    root.accessors.push(json::Accessor {
        buffer_view: Some(json::Index::new(root.buffer_views.len() as u32 - 1)),
        byte_offset: 0,
        count: indices.len() as u32,
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

    root.meshes.push(json::Mesh {
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        primitives: vec![json::mesh::Primitive {
            attributes: {
                let mut map = HashMap::new();
                map.insert(
                    Valid(json::mesh::Semantic::Positions),
                    json::Index::new(root.accessors.len() as u32 - 2),
                );
                map
            },
            extensions: Default::default(),
            extras: Default::default(),
            indices: Some(json::Index::new(root.accessors.len() as u32 - 1)),
            material: None,
            mode: Valid(json::mesh::Mode::Triangles),
            targets: None,
        }],
        weights: None,
    });

    root.nodes.push(json::Node {
        camera: None,
        children: None,
        extensions: Default::default(),
        extras: Default::default(),
        matrix: None,
        mesh: Some(json::Index::new(root.meshes.len() as u32 - 1)),
        name: None,
        rotation: None,
        scale: None,
        translation: None,
        skin: None,
        weights: None,
    });
}

pub fn gltf_from_skeleton(header: &SkeletonHeader, r: &mut RomReader) -> Result<json::Root> {
    let mut root: json::Root = Default::default();

    for limb in &header.limbs {
        match limb.skin_limb_type {
            Some(SkinLimbType::Animated(ref animated_limb)) => {
                let mut vertex_buffer =
                    vec![Vtx::default(); animated_limb.total_vtx_count as usize];

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
                        v.pos = [
                            vtx_point[0] as i16,
                            vtx_point[1] as i16,
                            vtx_point[2] as i16,
                        ];
                    }
                }

                log::info!(
                    "Animated limb vertex buffer constructed. len: {}",
                    vertex_buffer.len()
                );

                // FIXME: Need to make this big endian
                r.set_segment(
                    rom::Segment::IconItemStatic,
                    Some(bytemuck::cast_slice(&vertex_buffer).to_vec()),
                );

                log::info!("Animated limb display list @ 0x{:08X}", animated_limb.dlist);

                let DisplayListData { vertices, indices } =
                    dlist::gltf::dlist_to_gltf(r, animated_limb.dlist)?;

                create_mesh(&mut root, vertices, indices);
            }
            Some(SkinLimbType::Normal(dlist)) => {
                log::info!("Normal limb display list @ {}", dlist);

                let DisplayListData { vertices, indices } =
                    dlist::gltf::dlist_to_gltf(r, dlist.into())?;

                create_mesh(&mut root, vertices, indices);
            }
            None => (),
        }
    }

    root.scenes.push(json::Scene {
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        nodes: root
            .nodes
            .iter()
            .enumerate()
            .map(|(i, _)| json::Index::new(i as u32))
            .collect(),
    });

    Ok(root)
}
