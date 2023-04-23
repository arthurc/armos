use std::{collections::HashMap, mem};

use anyhow::{Context, Result};
use base64::prelude::*;
use gltf::json::{self, validation::Checked::Valid};
use zerocopy::AsBytes;

use crate::{
    display_list::{Instruction, Opcode, Tri1, Tri2, Vtx},
    rom,
};

#[derive(AsBytes, Debug)]
#[repr(C)]
pub struct Vertex {
    pub pos: [f32; 3],
}
impl Vertex {
    fn fold_pos(
        op: impl Fn(f32, f32) -> f32,
    ) -> impl FnMut(Option<[f32; 3]>, &Vertex) -> Option<[f32; 3]> {
        move |acc, v| match acc {
            None => Some(v.pos.clone()),
            Some([x, y, z]) => Some([op(x, v.pos[0]), op(y, v.pos[1]), op(z, v.pos[2])]),
        }
    }
}
impl From<&'_ rom::Vtx> for Vertex {
    fn from(rom::Vtx { pos, .. }: &rom::Vtx) -> Self {
        Self {
            pos: [pos[0].get() as _, pos[1].get() as _, pos[2].get() as _],
        }
    }
}

#[derive(Default, Debug)]
pub struct Mesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}
impl Mesh {
    pub fn write_into_gltf(&self, root: &mut json::Root) {
        root.buffers.push(json::Buffer {
            byte_length: mem::size_of_val(&*self.vertices) as _,
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            uri: Some(format!(
                "data:application/octet-stream;base64,{}",
                BASE64_STANDARD.encode(self.vertices.as_bytes())
            )),
        });
        root.buffer_views.push(json::buffer::View {
            buffer: json::Index::new(root.buffers.len() as u32 - 1),
            byte_length: mem::size_of_val(&*self.vertices) as _,
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
            count: self.vertices.len() as u32,
            component_type: Valid(json::accessor::GenericComponentType(
                json::accessor::ComponentType::F32,
            )),
            extensions: Default::default(),
            extras: Default::default(),
            type_: Valid(json::accessor::Type::Vec3),
            min: self.min_vertex_pos().map(|v| json::Value::from(v.to_vec())),
            max: self.max_vertex_pos().map(|v| json::Value::from(v.to_vec())),
            name: None,
            normalized: false,
            sparse: None,
        });

        root.buffers.push(json::Buffer {
            byte_length: mem::size_of_val(&*self.indices) as _,
            extensions: Default::default(),
            extras: Default::default(),
            name: None,
            uri: Some(format!(
                "data:application/octet-stream;base64,{}",
                BASE64_STANDARD.encode(self.indices.as_bytes())
            )),
        });
        root.buffer_views.push(json::buffer::View {
            buffer: json::Index::new(root.buffers.len() as u32 - 1),
            byte_length: mem::size_of_val(&*self.indices) as u32,
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
            count: self.indices.len() as u32,
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
    }

    fn min_vertex_pos(&self) -> Option<[f32; 3]> {
        self.vertices
            .iter()
            .fold(None, Vertex::fold_pos(|a, b| a.min(b)))
    }

    fn max_vertex_pos(&self) -> Option<[f32; 3]> {
        self.vertices
            .iter()
            .fold(None, Vertex::fold_pos(|a, b| a.max(b)))
    }
}

pub fn fold(reader: &rom::Reader) -> impl FnMut(Mesh, Instruction) -> Result<Mesh> + '_ {
    let mut vertex_offset = 0;
    move |mut mesh, instruction| {
        match instruction.opcode() {
            Opcode::VTX => {
                let data = Vtx::new(&instruction);
                vertex_offset = mesh.vertices.len();
                let vtxs = reader
                    .read_slice(data.addr(), data.nn() as _)
                    .context("Could not read vertices")?;

                for vtx in vtxs {
                    mesh.vertices.push(Vertex::from(vtx));
                }
            }
            Opcode::TRI1 => {
                let data = Tri1::new(&instruction);
                mesh.indices.push(vertex_offset as u32 + data.aa());
                mesh.indices.push(vertex_offset as u32 + data.bb());
                mesh.indices.push(vertex_offset as u32 + data.cc());
            }
            Opcode::TRI2 => {
                let data = Tri2::new(&instruction);
                mesh.indices.push(vertex_offset as u32 + data.aa());
                mesh.indices.push(vertex_offset as u32 + data.bb());
                mesh.indices.push(vertex_offset as u32 + data.cc());
                mesh.indices.push(vertex_offset as u32 + data.dd());
                mesh.indices.push(vertex_offset as u32 + data.ee());
                mesh.indices.push(vertex_offset as u32 + data.ff());
            }
            _ => (),
        }
        Ok(mesh)
    }
}
