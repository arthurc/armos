use crate::{rom::RomReader, skin::SkeletonHeader};
use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

mod dlist;
mod rom;
mod skin;

fn main() -> Result<()> {
    pretty_env_logger::init();

    let rom_path = get_rom_path()?;
    log::info!("Extracting assets from ROM file: {}", rom_path.display());

    let rom_file = fs::File::open(rom_path)?;
    let mut reader = RomReader::new(rom_file).with_segment(rom::Segment::Object, 0x010DB000);
    reader.seek(0x06009D74)?;

    let skeleton_header = SkeletonHeader::read(&mut reader)?;
    dbg!(&skeleton_header);

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
