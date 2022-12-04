use crate::{
    rom::{ReadSegment, RomReader},
    skeleton::{AnimationHeader, SkeletonAnimation},
    skin::SkeletonHeader,
};
use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

mod dlist;
mod rom;
mod skeleton;
mod skin;

fn main() -> Result<()> {
    pretty_env_logger::init();

    let rom_path = get_rom_path()?;
    log::info!("Extracting assets from ROM file: {}", rom_path.display());

    let rom_file = fs::File::open(rom_path)?;

    let mut reader = RomReader::new();
    // object_horse
    reader.set_segment_from(rom::Segment::Object, rom_file, (0x010DB000, 0x010E8F10))?;

    // gEponaSkel
    let skeleton_header = SkeletonHeader::read(reader.seek(0x06009D74))?;

    // gEponaGallopingAnim
    log::info!("Reading gEponaGallopingAnim");
    let animation_header = AnimationHeader::read(reader.seek(0x06001E2C))?;
    let _animation =
        SkeletonAnimation::create_from_header(&mut reader, &skeleton_header, &animation_header)?;

    log::info!("Writing skin gltf");
    let root = skin::gltf::gltf_from_skeleton(&skeleton_header, &mut reader)?;
    let writer = fs::File::create("skeleton.gltf").expect("I/O error");
    gltf::json::serialize::to_writer_pretty(writer, &root).expect("Serialization error");

    Ok(())
}

fn get_rom_path() -> Result<PathBuf> {
    let rom_path = glob::glob("*.z64")
        .expect("Failed to read glob pattern")
        .next()
        .with_context(|| "No ROM found")?
        .expect("Glob error");

    Ok(rom_path)
}
