use std::path::PathBuf;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    pretty_env_logger::init();

    let rom_file = get_rom_file()?;

    log::info!("Extracting assets from ROM file: {}", rom_file.display());

    Ok(())
}

fn get_rom_file() -> Result<PathBuf> {
    let rom_file = glob::glob("*.z64")
        .expect("Failed to read glob pattern")
        .next()
        .with_context(|| "No ROM found")?
        .expect("Glob error");

    Ok(rom_file)
}
