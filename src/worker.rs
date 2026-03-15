use anyhow::{Context, bail};
use rayon::prelude::*;

use crate::palette::LUT;
use crate::{image_processing, resolution::Resolutions};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub struct ProcessTask {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub res: Resolutions,
}

pub fn single_file(
    input_path: &Path,
    output_path: &Path,
    res: Resolutions,
    lut: &LUT<u8>,
) -> anyhow::Result<()> {
    if !input_path.is_file() {
        bail!("Error: Input file {:?} is not a file.", input_path);
    }

    if let Some(parent_dir) = output_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)
                .with_context(|| format!("Failed to create directory {}", parent_dir.display()))?;
        }
    }

    image_processing::process_image(lut, input_path, output_path, res)
        .with_context(|| "Failed to process image")
}

pub fn multi_file(
    input_path: &Path,
    output_path: &Path,
    lut: LUT<u8>,
    res: Resolutions,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        input_path.is_dir(),
        "Input {} does not exist or is not a directory",
        input_path.display()
    );
    anyhow::ensure!(
        input_path.is_dir(),
        "Output {} does not exist or is not a directory",
        input_path.display()
    );

    let mut queue = Vec::<ProcessTask>::new();

    for entry in fs::read_dir(input_path)? {
        let Ok(entry) = entry else {
            continue;
        };

        let input_file = entry.path();
        if !input_file.is_file() {
            continue;
        }

        let mut output_file = output_path.join(entry.file_name());

        if let Some(file_name) = output_file.file_name() {
            let new_file_name = format!("palettify-{}", file_name.to_string_lossy());
            output_file.set_file_name(new_file_name);
        }

        println!("Added {} to queue", input_file.display());
        queue.push(ProcessTask {
            input_path: input_file.clone(),
            output_path: output_file.clone(),
            res,
        });
    }

    batch_process(queue, lut);
    Ok(())
}

fn batch_process(queue: Vec<ProcessTask>, lut: LUT<u8>) {
    let lut = &lut; // plain shared ref, LUT<u8> is Sync
    let errors: Vec<_> = queue
        .par_iter()
        .filter_map(|task| {
            println!("Processing {}...", task.input_path.display());
            image_processing::process_image(lut, &task.input_path, &task.output_path, task.res)
                .err()
                .map(|e| format!("Failed to process {}: {e}", task.input_path.display()))
        })
        .collect();

    for e in errors {
        eprintln!("{e}");
    }
}
