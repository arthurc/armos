use crate::rom::VirtualAddress;

use super::*;

use anyhow::Result;
use bytemuck::{Pod, Zeroable};

#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub pos: [f32; 3],
}

pub fn fold_pos(vtxs: &[Vertex], op: impl Fn(f32, f32) -> f32) -> Option<[f32; 3]> {
    vtxs.iter().fold(None, |acc, v| match acc {
        None => Some(v.pos.clone()),
        Some([x, y, z]) => Some([op(x, v.pos[0]), op(y, v.pos[1]), op(z, v.pos[2])]),
    })
}

struct Interpreter<'a> {
    iter: InstrIter,
    r: &'a mut RomReader,
    tris: Vec<[u32; 3]>,
    current_vtx_addr: Option<VirtualAddress>,
    done: bool,

    indices: Vec<u32>,
    vertices: Vec<Vertex>,
}
impl<'a> Interpreter<'a> {
    fn new(addr: VirtualAddress, r: &'a mut RomReader) -> Self {
        Self {
            iter: InstrIter::new(addr),
            r,
            tris: Vec::new(),
            current_vtx_addr: None,
            done: false,
            indices: Vec::new(),
            vertices: Vec::new(),
        }
    }
    fn next(&mut self) -> Result<bool> {
        if self.done {
            return Ok(false);
        }

        let Some((opcode, data)) = self.iter.next(self.r)? else {
            self.done = true;
            self.flush(None)?;
            return Ok(true);
        };

        match opcode {
            Opcode::VTX => {
                let nn = ((data & 0x000FF00000000000u64) >> 44) as u32;
                let aa = ((data & 0x000000FF00000000u64) >> 32) as u32;
                let addr = VirtualAddress::new(data as u32);

                log::trace!(
                    "VTX nn: {} aa: {} (aa >> 1) - nn: {} addr: {}",
                    nn,
                    aa,
                    (aa >> 1) - nn,
                    addr
                );

                self.flush(Some(addr))?;
                return Ok(true);
            }
            Opcode::TRI1 => {
                // let aa = (((data & 0x0000000000FF0000u64) >> 16) / 2) as u32;
                // let bb = (((data & 0x000000000000FF00u64) >> 8) / 2) as u32;
                // let cc = (((data & 0x00000000000000FFu64) >> 0) / 2) as u32;

                let aa = (((data & 0x00FF000000000000u64) >> 48) / 2) as u32;
                let bb = (((data & 0x0000FF0000000000u64) >> 40) / 2) as u32;
                let cc = (((data & 0x000000FF00000000u64) >> 32) / 2) as u32;

                log::trace!("TRI1 aa: {} bb: {} cc: {}", aa, bb, cc,);

                self.tris.push([aa as _, bb as _, cc as _]);
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

                self.tris.push([aa as _, bb as _, cc as _]);
                self.tris.push([dd as _, ee as _, ff as _]);
            }
            _ => (),
        }

        Ok(true)
    }

    fn flush(&mut self, next_vtx_addr: Option<VirtualAddress>) -> Result<()> {
        let Some(vtx_addr) = mem::replace(&mut self.current_vtx_addr, next_vtx_addr) else {
            return Ok(());
        };
        let Some(max_tri_index) = self.tris.iter().flat_map(|&v| v.into_iter()).max() else {
            return Ok(());
        };

        let indices_start_index = self.vertices.len() as u32;

        for vertex in self
            .r
            .segment_iter::<Vtx>(vtx_addr)
            .take(max_tri_index as usize + 1)
            .map(|vtx| {
                vtx.map(|vtx| Vertex {
                    pos: [vtx.pos[0] as _, vtx.pos[1] as _, vtx.pos[2] as _],
                })
            })
        {
            self.vertices.push(vertex?);
        }

        self.tris.drain(..).for_each(|tri| {
            self.indices.push(indices_start_index + tri[0]);
            self.indices.push(indices_start_index + tri[1]);
            self.indices.push(indices_start_index + tri[2]);
        });

        Ok(())
    }
}

pub struct DisplayListData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

pub fn dlist_to_gltf(r: &mut RomReader, addr: VirtualAddress) -> Result<DisplayListData> {
    let mut interpreter = Interpreter::new(addr, r);
    while interpreter.next()? {}

    Ok(DisplayListData {
        vertices: interpreter.vertices,
        indices: interpreter.indices,
    })
}
