use std::collections::HashMap;
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

pub fn build_image_output_target(
    output_dir: &Path,
    date: NaiveDate,
    output_file_name: Option<&str>,
) -> ImageOutputTarget {
    let file_name = output_file_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| default_output_file_name(date));

    ImageOutputTarget {
        images_csv: output_dir.join(file_name),
    }
}

fn default_output_file_name(date: NaiveDate) -> String {
    let stamp = date.format("%Y%m%d").to_string();
    format!("images_{stamp}.csv")
}

pub fn write_image_csv(path: &Path, rows: &[ImageRow]) -> Result<usize> {
    let file = create_utf8_bom_file(path)?;
    let mut writer = csv::Writer::from_writer(file);
    let grouped_rows = group_rows_by_sku(rows);

    writer
        .write_record(["FileName", "Sku"])
        .with_context(|| format!("Failed writing headers to {}", path.display()))?;

    for row in &grouped_rows {
        writer
            .write_record([row.file_name.as_str(), row.sku.as_str()])
            .with_context(|| format!("Failed writing row to {}", path.display()))?;
    }

    writer
        .flush()
        .with_context(|| format!("Failed flushing writer for {}", path.display()))?;

    Ok(grouped_rows.len())
}

fn group_rows_by_sku(rows: &[ImageRow]) -> Vec<ImageRow> {
    let mut sku_indexes: HashMap<String, usize> = HashMap::new();
    let mut grouped_rows: Vec<ImageRow> = Vec::new();

    for row in rows {
        if let Some(index) = sku_indexes.get(&row.sku) {
            grouped_rows[*index].file_name.push('|');
            grouped_rows[*index].file_name.push_str(&row.file_name);
        } else {
            let index = grouped_rows.len();
            sku_indexes.insert(row.sku.clone(), index);
            grouped_rows.push(ImageRow {
                file_name: row.file_name.clone(),
                sku: row.sku.clone(),
            });
        }
    }

    grouped_rows
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

#[cfg(test)]
mod tests {
    use super::{build_image_output_target, group_rows_by_sku};
    use crate::core::ImageRow;
    use chrono::NaiveDate;
    use std::path::Path;

    #[test]
    fn groups_multiple_images_for_same_sku_into_one_row() {
        let rows = vec![
            ImageRow {
                file_name: r"\\etail.rs\files\hardwaretools\Images2\Images_Uploaded_3_18_26\HT-033923820684.jpg".to_owned(),
                sku: "HT-033923820684".to_owned(),
            },
            ImageRow {
                file_name: r"\\etail.rs\files\hardwaretools\Images2\Images_Uploaded_3_18_26\HT-033923820684-1.jpg".to_owned(),
                sku: "HT-033923820684".to_owned(),
            },
            ImageRow {
                file_name: r"\\etail.rs\files\hardwaretools\Images2\Images_Uploaded_3_18_26\HT-033923820684-2.jpg".to_owned(),
                sku: "HT-033923820684".to_owned(),
            },
        ];

        let grouped = group_rows_by_sku(&rows);

        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].sku, "HT-033923820684");
        assert_eq!(
            grouped[0].file_name,
            r"\\etail.rs\files\hardwaretools\Images2\Images_Uploaded_3_18_26\HT-033923820684.jpg|\\etail.rs\files\hardwaretools\Images2\Images_Uploaded_3_18_26\HT-033923820684-1.jpg|\\etail.rs\files\hardwaretools\Images2\Images_Uploaded_3_18_26\HT-033923820684-2.jpg"
        );
    }

    #[test]
    fn preserves_first_seen_sku_order() {
        let rows = vec![
            ImageRow {
                file_name: "first-a.jpg".to_owned(),
                sku: "SKU-A".to_owned(),
            },
            ImageRow {
                file_name: "first-b.jpg".to_owned(),
                sku: "SKU-B".to_owned(),
            },
            ImageRow {
                file_name: "second-a.jpg".to_owned(),
                sku: "SKU-A".to_owned(),
            },
        ];

        let grouped = group_rows_by_sku(&rows);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].sku, "SKU-A");
        assert_eq!(grouped[0].file_name, "first-a.jpg|second-a.jpg");
        assert_eq!(grouped[1].sku, "SKU-B");
        assert_eq!(grouped[1].file_name, "first-b.jpg");
    }

    #[test]
    fn uses_default_dated_output_name_when_none_provided() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 25).expect("valid test date");
        let target = build_image_output_target(Path::new("/tmp/out"), date, None);

        assert_eq!(target.images_csv, Path::new("/tmp/out/images_20260325.csv"));
    }

    #[test]
    fn uses_custom_output_name_when_provided() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 25).expect("valid test date");
        let target = build_image_output_target(Path::new("/tmp/out"), date, Some("custom.csv"));

        assert_eq!(target.images_csv, Path::new("/tmp/out/custom.csv"));
    }
}
