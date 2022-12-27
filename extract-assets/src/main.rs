use crate::rom::RomReader;
use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

mod dlist;
mod n64;
mod rom;
mod skin;

fn main() -> Result<()> {
    pretty_env_logger::init();

    let rom_path = get_rom_path()?;
    log::info!("Extracting assets from ROM file: {}", rom_path.display());

    let rom_file = fs::File::open(rom_path)?;

    let mut reader = RomReader::new();
    // object_horse
    reader.set_segment_from(rom::Segment::Object, rom_file, (0x010DB000, 0x010E8F10))?;

    let mut epona = skin::Skin::read(&mut reader, 0x06009D74)?;
    epona.read_animation("gEponaGallopingAnim", &mut reader, 0x06001E2C)?;
    epona.read_animation("gEponaJumpingAnim", &mut reader, 0x06002470)?;

    let root = epona.to_gltf(&mut reader)?;
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
