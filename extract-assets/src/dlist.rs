use std::mem;

use anyhow::{anyhow, Result};
use bytemuck::{Pod, Zeroable};

use crate::rom::{ReadSegment, RomReader};

pub mod gltf;

#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vtx {
    pub pos: [i16; 3],
    pub flag: i16,
    pub tpos: [i16; 2],
    pub cn: [u8; 4],
}
impl ReadSegment for Vtx {
    const SIZE: u32 = 16;

    fn read(r: &mut RomReader) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            pos: [r.read_i16()?, r.read_i16()?, r.read_i16()?],
            ..Default::default()
        })
    }
}

#[derive(Debug)]
pub struct InstrIter {
    pos: u32,
}
impl InstrIter {
    pub fn new(addr: u32) -> Self {
        Self { pos: addr }
    }

    pub fn next(&mut self, r: &mut RomReader) -> Result<Option<(Opcode, u64)>> {
        r.seek(self.pos as _);
        let data = r.read_u64()?;
        self.pos += mem::size_of::<u64>() as u32;
        let opcode = Opcode::from_data(data)?;

        log::trace!(
            "Opcode: {: <20} Data: 0x{:016X}",
            format!("{:?}", opcode),
            data
        );

        if opcode == Opcode::ENDDL {
            Ok(None)
        } else {
            Ok(Some((opcode, data)))
        }
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub enum Opcode {
    ENDDL,
    GEOMETRYMODE,
    LOADBLOCK,
    LOADTLUT,
    RDPLOADSYNC,
    RDPPIPESYNC,
    RDPTILESYNC,
    SETCOMBINE,
    SETPRIMCOLOR,
    SETTILE,
    SETTILESIZE,
    SETTIMG,
    SETOTHERMODE_H,
    SETOTHERMODE_L,
    TEXTURE,
    TRI1,
    TRI2,
    VTX,
}
impl Opcode {
    fn from_data(data: u64) -> Result<Opcode> {
        use Opcode::*;

        let op = data >> 56;

        match op {
            0x01 => Ok(VTX),
            0x05 => Ok(TRI1),
            0x06 => Ok(TRI2),
            0xD7 => Ok(TEXTURE),
            0xD9 => Ok(GEOMETRYMODE),
            0xDF => Ok(ENDDL),
            0xE2 => Ok(SETOTHERMODE_L),
            0xE3 => Ok(SETOTHERMODE_H),
            0xE6 => Ok(RDPLOADSYNC),
            0xE7 => Ok(RDPPIPESYNC),
            0xE8 => Ok(RDPTILESYNC),
            0xF0 => Ok(LOADTLUT),
            0xF2 => Ok(SETTILESIZE),
            0xF3 => Ok(LOADBLOCK),
            0xF5 => Ok(SETTILE),
            0xFA => Ok(SETPRIMCOLOR),
            0xFC => Ok(SETCOMBINE),
            0xFD => Ok(SETTIMG),
            _ => Err(anyhow!(
                "Unknown opcode 0x{:02X} from data 0x{:016X}",
                op,
                data
            )),
        }
    }
}
