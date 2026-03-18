use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::NaiveDate;

use crate::core::ImageRow;

#[derive(Debug, Clone)]
pub struct ImageOutputTarget {
    pub images_csv: PathBuf,
}

pub fn build_image_output_target(output_dir: &Path, date: NaiveDate) -> ImageOutputTarget {
    let stamp = date.format("%Y%m%d").to_string();
    ImageOutputTarget {
        images_csv: output_dir.join(format!("images_{stamp}.csv")),
    }
}

pub fn write_image_csv(path: &Path, rows: &[ImageRow]) -> Result<usize> {
    let file = create_utf8_bom_file(path)?;
    let mut writer = csv::Writer::from_writer(file);

    writer
        .write_record(["FileName", "Sku"])
        .with_context(|| format!("Failed writing headers to {}", path.display()))?;

    for row in rows {
        writer
            .write_record([row.file_name.as_str(), row.sku.as_str()])
            .with_context(|| format!("Failed writing row to {}", path.display()))?;
    }

    writer
        .flush()
        .with_context(|| format!("Failed flushing writer for {}", path.display()))?;

    Ok(rows.len())
}

fn create_utf8_bom_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed creating directory {}", parent.display()))?;
    }

    let mut file = File::create(path)
        .with_context(|| format!("Failed creating output file {}", path.display()))?;
    file.write_all(&[0xEF, 0xBB, 0xBF])
        .with_context(|| format!("Failed writing UTF-8 BOM to {}", path.display()))?;
    Ok(file)
}
