mod cli;
mod image_processing;
mod palette;
mod resolution;
mod worker;
use std::time::Instant;

use anyhow::bail;
use clap::Parser;
use cli::Cli;

use crate::{
    palette::{LUT, Palette},
    worker::single_file,
};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    println!("Res {:?}", cli.resolution);

    if !cli.input_path.exists() {
        bail!("Error: Input file {:?} does not exist.", cli.input_path);
    } else if !cli.palette_path.exists() {
        bail!("Error: Palette file {:?} does not exist.", cli.palette_path);
    } else if !cli.palette_path.is_file() {
        bail!("Error: Palette file {:?} is not a file.", cli.palette_path);
    }

    let palette = Palette::read(&cli.palette_path)?;
    let lut = LUT::from_palette(palette);

    if cli.dir {
        let elapsed = timed!(worker::multi_file(
            &cli.input_path,
            &cli.output_path,
            lut,
            cli.resolution
        )?);
        println!("Task took {} seconds", elapsed.as_secs())
    } else {
        let elapsed = timed!(single_file(
            &cli.input_path,
            &cli.output_path,
            cli.resolution,
            &lut
        )?);
        println!("Task took {} seconds", elapsed.as_secs())
    }
    Ok(())
}

#[macro_export]
macro_rules! timed {
    ( $( $x:expr ),* ) => {
        {
            let now = Instant::now();
            $(
                $x;
            )*
            let elapsed = now.elapsed();
            elapsed
        }
    };
}
