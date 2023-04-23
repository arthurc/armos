use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use gltf::json;

use crate::addr::RawVirtAddr;

mod addr;
mod display_list;
mod math;
mod mesh;
mod rom;
mod skeleton;
mod skeleton_animation;

fn main() -> Result<()> {
    pretty_env_logger::init();

    let rom_path = get_rom_path()?;
    let mut rom_file = fs::File::open(rom_path)?;

    let mut reader = rom::Reader::new();
    reader.read_segment(rom::Segment::Object, &mut rom_file, 0x010DB000..0x010E8F10)?;

    let mut root = gltf::json::Root::default();
    skeleton::read_into_gltf(
        &mut root,
        &reader,
        RawVirtAddr::new(0x06009D74).into(),
        &[
            // gEponaGallopingAnim
            RawVirtAddr::new(0x06001E2C).into(),
            // gEponaJumpingAnim
            RawVirtAddr::new(0x06002470).into(),
        ],
    )?;

    root.scenes.push(json::Scene {
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        nodes: vec![json::Index::new(0)],
    });

    let writer = fs::File::create("epona.gltf")?;
    gltf::json::serialize::to_writer_pretty(writer, &root)?;

    Ok(())
}

fn get_rom_path() -> Result<PathBuf> {
    Ok(glob::glob("*.z64")
        .expect("Failed to read glob pattern")
        .next()
        .with_context(|| "No ROM found")?
        .expect("Glob error"))
}
