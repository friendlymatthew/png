use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Parser;
use minifb::{Window, WindowOptions};
use png::Decoder;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    image_path: PathBuf,
}

fn main() -> Result<()> {
    let Args { image_path } = Args::parse();

    let image_path = image_path
        .to_str()
        .ok_or(anyhow!("Failed to find file {:?} to render.", image_path))?;

    let content = std::fs::read(image_path)?;
    let mut decoder = Decoder::new(&content);
    let png = decoder.decode()?;

    let (width, height) = png.dimension();

    let mut window = Window::new(
        "PNG renderer",
        width as usize,
        height as usize,
        WindowOptions::default(),
    )?;

    while window.is_open() {
        window.update_with_buffer(&png.pixel_buffer(), width as usize, height as usize)?;
    }

    Ok(())
}
