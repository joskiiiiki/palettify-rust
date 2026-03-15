// src/cli.rs
use clap::Parser;
use std::path::PathBuf;

use crate::resolution::Resolutions;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The input image file path
    #[arg(short, long, value_name = "INPUT", help = "Path to the input image")]
    pub input_path: PathBuf,

    /// The output image file path
    #[arg(
        short,
        long,
        value_name = "OUTPUT",
        default_value = "o",
        help = "Path to the output image (default: o)"
    )]
    pub output_path: PathBuf,

    /// The palette file path
    #[arg(short, long, value_name = "PALETTE", help = "Path to the palette file")]
    pub palette_path: PathBuf,

    /// The exponent value for processing (default: 2)
    #[arg(
        short,
        long,
        value_name = "EXPONENT",
        default_value_t = 15,
        help = "Exponent for processing. Bigger Exponent > more quantization (default: 15)"
    )]
    pub exponent: i32,

    #[arg(short, long, value_name = "RESOLUTION", default_value_t = Resolutions::NONE, help = "Rescales the image to the given width")]
    pub resolution: Resolutions,

    #[arg(long, short, action)]
    pub dir: bool,
}
