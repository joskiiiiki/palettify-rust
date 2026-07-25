use crate::interpolation::Interpolation;
use crate::resolution::Resolutions;
use crate::{interpolation as intp, palette::Palette};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to the palette file
    #[arg(value_name = "PALETTE")]
    pub palette_path: PathBuf,
    /// Path to the input image
    #[arg(value_name = "INPUT")]
    pub input_path: PathBuf,
    /// Path to the output image
    #[arg(
        value_name = "OUTPUT",
        default_value = "o",
        global = true,
        help = "Path to the output image (default: o)"
    )]
    pub output_path: PathBuf,
    #[arg(short, long, value_name = "RESOLUTION", default_value_t = Resolutions::NONE, global = true, help = "Rescales the image to the given width")]
    pub resolution: Resolutions,
    #[command(subcommand)]
    pub algorithm: Algorithm,
}

#[derive(Subcommand, Debug)]
pub enum Algorithm {
    /// Inverse-distance weighting in raw RGB space
    RgbEuclid {
        #[arg(
            short,
            long,
            default_value_t = 15.0,
            help = "Bigger exponent > more quantization"
        )]
        exponent: f32,
    },
    /// Inverse-distance weighting in perceptual OKLab space
    OklabEuclid {
        #[arg(
            short,
            long,
            default_value_t = 15.0,
            help = "Bigger exponent > more quantization"
        )]
        exponent: f32,
    },
    /// Softmin (exponential) weighting in perceptual OKLab space
    OklabSoftmin {
        #[arg(
            short,
            long,
            default_value_t = 0.1,
            help = "Smaller = sharper/nearest-neighbor-like, larger = smoother/more averaged"
        )]
        temperature: f32,
    },
    /// Radial basis function interpolation in raw RGB space
    Rbf {
        #[arg(
            short,
            long,
            default_value_t = 8.0,
            help = "RBF shape parameter. Larger = sharper/more local, smaller = smoother/more global"
        )]
        epsilon: f32,
    },
    /// Radial basis function interpolation in perceptual OKLab space
    OklabRbf {
        #[arg(
            short,
            long,
            default_value_t = 8.0,
            help = "RBF shape parameter. Larger = sharper/more local, smaller = smoother/more global"
        )]
        epsilon: f32,
    },
    /// Base16-aware: separates shades (0-7) from accents (8-15), blends each group separately
    Base16RoleAware {
        #[arg(
            long,
            default_value_t = 0.15,
            help = "Softmin temperature for shade group"
        )]
        shade_temperature: f32,
        #[arg(
            long,
            default_value_t = 0.03,
            help = "Softmin temperature for accent group"
        )]
        accent_temperature: f32,
    },
    /// Base16-aware: separates shades (0-7) from accents (8-15), tints shadows with accent hue
    Base16Tinted {
        #[arg(
            long,
            default_value_t = 0.15,
            help = "Softmin temperature for shade group"
        )]
        shade_temperature: f32,
        #[arg(
            long,
            default_value_t = 0.03,
            help = "Softmin temperature for accent group"
        )]
        accent_temperature: f32,
        #[arg(
            long,
            default_value_t = 0.01,
            help = "Softmin temperature for tint group"
        )]
        tint_temperature: f32,
        #[arg(
            long,
            default_value_t = 1.5,
            help = "How quickly shade tint ramps with input chroma"
        )]
        tint_gain: f32,
        #[arg(
            long,
            default_value_t = 0.35,
            help = "Cap on accent-hue bleed into shades"
        )]
        max_tint: f32,
    },

    /// Base16-aware with RBF-smoothed shade/accent groups instead of softmin
    Base16TintedRbf {
        #[arg(
            long,
            default_value_t = 0.01,
            help = "Softmin temperature for tint hue source"
        )]
        tint_temperature: f32,
        #[arg(
            long,
            default_value_t = 1.5,
            help = "How quickly shade tint ramps with input chroma"
        )]
        tint_gain: f32,
        #[arg(
            long,
            default_value_t = 0.35,
            help = "Cap on accent-hue bleed into shades"
        )]
        max_tint: f32,
        #[arg(
            short,
            long,
            default_value_t = 8.0,
            help = "RBF shape parameter for shade/accent group fits. Larger = sharper/more local, smaller = smoother/more global"
        )]
        epsilon: f32,
    },
}

pub fn build_interpolation(cli: &Cli, palette: Vec<[f32; 3]>) -> Box<dyn intp::Interpolation> {
    match cli.algorithm {
        Algorithm::RgbEuclid { exponent } => {
            Box::new(intp::RgbEuclidDistance::new(exponent).with(palette))
        }
        Algorithm::OklabEuclid { exponent } => {
            Box::new(intp::OkLabEuclidDistance::new(exponent).with(palette))
        }
        Algorithm::OklabSoftmin { temperature } => {
            Box::new(intp::OkLabSoftmin::new(temperature).with(palette))
        }
        Algorithm::Rbf { epsilon } => Box::new(intp::RbfInterpolation::new(epsilon).with(palette)),
        Algorithm::OklabRbf { epsilon } => {
            Box::new(intp::OkLabRbfInterpolation::new(epsilon).with(palette))
        }
        Algorithm::Base16RoleAware {
            shade_temperature,
            accent_temperature,
        } => {
            assert!(
                palette.len() == 16,
                "base16-role-aware requires exactly 16 palette entries"
            );
            Box::new(
                intp::Base16RoleAware::new(shade_temperature, accent_temperature).with(palette),
            )
        }
        Algorithm::Base16Tinted {
            shade_temperature,
            accent_temperature,
            tint_temperature,
            tint_gain,
            max_tint,
        } => {
            assert!(
                palette.len() == 16,
                "base16-tinted requires exactly 16 palette entries"
            );
            Box::new(
                intp::Base16Tinted::new(
                    shade_temperature,
                    accent_temperature,
                    tint_temperature,
                    tint_gain,
                    max_tint,
                )
                .with(palette),
            )
        }

        Algorithm::Base16TintedRbf {
            tint_temperature,
            tint_gain,
            max_tint,
            epsilon,
        } => {
            assert!(
                palette.len() == 16,
                "base16-tinted-rbf requires exactly 16 palette entries"
            );
            Box::new(
                intp::Base16TintedRbf::new(tint_temperature, tint_gain, max_tint, epsilon)
                    .with(palette),
            )
        }
    }
}
