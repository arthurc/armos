use std::fmt::{Debug, Display};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use zerocopy::{BigEndian, LayoutVerified, U64};

use crate::{
    addr::{RawVirtAddr, VirtAddr},
    rom,
};

pub fn dump() -> impl FnMut(Instruction) -> () {
    |instruction| log::trace!(target: "display_list::dump", "  {:?}", instruction)
}

#[derive(Copy, Clone, FromPrimitive, Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Opcode {
    VTX = 0x01,
    TRI1 = 0x05,
    TRI2 = 0x06,
    TEXTURE = 0xD7,
    GEOMETRYMODE = 0xD9,
    ENDDL = 0xDF,
    SETOTHERMODE_L = 0xE2,
    SETOTHERMODE_H = 0xE3,
    RDPLOADSYNC = 0xE6,
    RDPPIPESYNC = 0xE7,
    RDPTILESYNC = 0xE8,
    LOADTLUT = 0xF0,
    SETTILESIZE = 0xF2,
    LOADBLOCK = 0xF3,
    SETTILE = 0xF5,
    SETPRIMCOLOR = 0xFA,
    SETCOMBINE = 0xFC,
    SETTIMG = 0xFD,
}

pub struct Instruction(u64);
impl Instruction {
    pub fn new(data: u64) -> Self {
        let instruction = Self(data);
        let _ = instruction.opcode();
        instruction
    }

    pub fn opcode(&self) -> Opcode {
        let opcode = self.0 >> 56;
        match Opcode::from_u64(opcode) {
            Some(opcode) => opcode,
            _ => panic!("Unknown opcode: {:#04X}", opcode),
        }
    }
}
impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let opcode = self.opcode();

        write!(f, "{:?}", opcode)?;

        match opcode {
            Opcode::VTX => write!(f, " {:?}", Vtx::new(self))?,
            Opcode::TRI1 => write!(f, " {:?}", Tri1::new(self))?,
            Opcode::TRI2 => write!(f, " {:?}", Tri2::new(self))?,
            _ => (),
        }

        Ok(())
    }
}
impl Debug for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:#018X}) {}", self.0, self)
    }
}

#[derive(Clone)]
pub struct InstructionStream<'a>(&'a [u8]);
impl<'a> InstructionStream<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self(data)
    }
}
impl Iterator for InstructionStream<'_> {
    type Item = Instruction;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            return None;
        }

        let (lv, rest) = LayoutVerified::<_, U64<BigEndian>>::new_from_prefix(self.0)?;
        let instruction = Instruction::new(lv.read().get());
        self.0 = if instruction.opcode() == Opcode::ENDDL {
            &[]
        } else {
            rest
        };
        Some(instruction)
    }
}

pub struct Vtx(u64);
impl Vtx {
    pub fn new(instruction: &Instruction) -> Self {
        Self(instruction.0)
    }

    pub fn addr(&self) -> VirtAddr<rom::Vtx> {
        RawVirtAddr::new(self.0 as _).into()
    }

    pub fn nn(&self) -> u32 {
        ((self.0 & 0x000FF00000000000u64) >> 44) as _
    }
    pub fn aa(&self) -> u32 {
        ((self.0 & 0x000000FF00000000u64) >> 32) as _
    }
}
impl Debug for Vtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "addr:{} nn:{} aa:{}", self.addr(), self.nn(), self.aa())
    }
}

pub struct Tri1(u64);
impl Tri1 {
    pub fn new(instruction: &Instruction) -> Self {
        Self(instruction.0)
    }

    pub fn aa(&self) -> u32 {
        (((self.0 & 0x00FF000000000000u64) >> 48) / 2) as _
    }
    pub fn bb(&self) -> u32 {
        (((self.0 & 0x0000FF0000000000u64) >> 40) / 2) as _
    }
    pub fn cc(&self) -> u32 {
        (((self.0 & 0x000000FF00000000u64) >> 32) / 2) as _
    }
}
impl Debug for Tri1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "aa:{} bb:{} cc:{}", self.aa(), self.bb(), self.cc())
    }
}

pub struct Tri2(u64);
impl Tri2 {
    pub fn new(instruction: &Instruction) -> Self {
        Self(instruction.0)
    }
    pub fn aa(&self) -> u32 {
        (((self.0 & 0x00FF000000000000u64) >> 48) / 2) as _
    }
    pub fn bb(&self) -> u32 {
        (((self.0 & 0x0000FF0000000000u64) >> 40) / 2) as _
    }
    pub fn cc(&self) -> u32 {
        (((self.0 & 0x000000FF00000000u64) >> 32) / 2) as _
    }

    pub fn dd(&self) -> u32 {
        (((self.0 & 0x0000000000FF0000u64) >> 16) / 2) as _
    }
    pub fn ee(&self) -> u32 {
        (((self.0 & 0x000000000000FF00u64) >> 8) / 2) as _
    }
    pub fn ff(&self) -> u32 {
        (((self.0 & 0x00000000000000FFu64) >> 0) / 2) as _
    }
}

impl Debug for Tri2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "aa:{} bb:{} cc:{} dd:{} ee:{} ff:{}",
            self.aa(),
            self.bb(),
            self.cc(),
            self.dd(),
            self.ee(),
            self.ff()
        )
    }
}
