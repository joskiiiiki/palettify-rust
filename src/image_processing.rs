use image::{ImageReader, Rgba};
use rayon::iter::ParallelIterator;
use std::{cmp::min, path::Path};

use crate::{palette::LUT, resolution::Resolutions};

pub type Image = image::ImageBuffer<Rgba<u8>, Vec<u8>>;

fn decode_image(path: &Path) -> anyhow::Result<Image> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => {
            let data = std::fs::read(path)?;
            let img: Image = turbojpeg::decompress_image(&data)?;
            Ok(img)
        }
        _ => Ok(ImageReader::open(path)?.decode()?.into_rgba8()),
    }
}

fn encode_image(img: &Image, path: &Path) -> anyhow::Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => {
            let data = turbojpeg::compress_image(img, 90, turbojpeg::Subsamp::Sub2x2)?;
            std::fs::write(path, &data)?;
        }
        _ => img.save(path)?,
    }
    Ok(())
}
pub fn process_image(
    lut: &LUT<u8>,
    input_path: &Path,
    output_path: &Path,
    resolution: Resolutions,
) -> anyhow::Result<()> {
    let mut img = decode_image(input_path)?;

    if let Some(resized) = img_resize(&img, resolution) {
        img = resized;
    }

    process(lut, &mut img);

    encode_image(&img, output_path)?;

    Ok(())
}

use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};

fn img_resize(image: &Image, res: Resolutions) -> Option<Image> {
    if res == Resolutions::NONE {
        return None;
    }
    let res = res as u32;
    let (width, height) = image.dimensions();
    let min_dim = min(width, height);

    if min_dim < res {
        return None;
    }

    let f = res as f32 / min_dim as f32;
    let nwidth = ((width as f32 * f) as u32).max(1);
    let nheight = ((height as f32 * f) as u32).max(1);

    let mut dst = Image::new(nwidth, nheight);
    let mut resizer = Resizer::new();
    resizer
        .resize(
            image,
            &mut dst,
            &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Bilinear)),
        )
        .unwrap();

    Some(dst)
}
pub fn process(lut: &LUT<u8>, img: &mut Image) {
    img.par_pixels_mut().for_each(|pixel| {
        let [r, g, b, _] = &mut pixel.0;
        let [nr, ng, nb] = lut.lookup(*r, *g, *b);
        *r = nr;
        *g = ng;
        *b = nb;
    });
}
