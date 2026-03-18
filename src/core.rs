use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct ImageRow {
    pub file_name: String,
    pub sku: String,
}

#[derive(Debug, Clone)]
pub struct ImageCollection {
    pub image_rows: Vec<ImageRow>,
    pub secondary_only_images: Vec<String>,
    pub secondary_only_sku_count: usize,
}

pub fn parent_sku_from_filename(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);

    let (parent, suffix) = match stem.rsplit_once('-') {
        Some(parts) => parts,
        None => return stem.to_owned(),
    };

    if parent.contains('-') && suffix.chars().all(|character| character.is_ascii_digit()) {
        parent.to_owned()
    } else {
        stem.to_owned()
    }
}

pub fn collect_image_rows(image_folder: &Path, unc_base: &str) -> Result<ImageCollection> {
    if !image_folder.is_dir() {
        anyhow::bail!(
            "Image folder is not a directory: {}",
            image_folder.display()
        );
    }

    let image_names = list_jpg_names(image_folder)?;
    let normalized_unc = unc_base.trim().trim_end_matches('\\');

    if normalized_unc.is_empty() {
        anyhow::bail!("UNC base path cannot be blank");
    }

    let image_rows = image_names
        .iter()
        .map(|image_name| ImageRow {
            file_name: format!(r"{}\{}", normalized_unc, image_name),
            sku: parent_sku_from_filename(image_name),
        })
        .collect::<Vec<_>>();

    let (secondary_only_sku_count, secondary_only_images) = secondary_only_summary(&image_names);

    Ok(ImageCollection {
        image_rows,
        secondary_only_images,
        secondary_only_sku_count,
    })
}

fn list_jpg_names(directory: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();

    for entry in directory
        .read_dir()
        .with_context(|| format!("Failed listing {}", directory.display()))?
    {
        let entry =
            entry.with_context(|| format!("Failed reading entry in {}", directory.display()))?;
        let path = entry.path();

        if path.is_file()
            && path
                .extension()
                .and_then(|value| value.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("jpg"))
                .unwrap_or(false)
        {
            if let Some(file_name) = path.file_name().and_then(|value| value.to_str()) {
                names.push(file_name.to_owned());
            }
        }
    }

    sort_image_names(&mut names);
    Ok(names)
}

fn sort_image_names(names: &mut [String]) {
    names.sort_by(|left, right| {
        let left_key = sort_key(left);
        let right_key = sort_key(right);
        left_key.cmp(&right_key)
    });
}

fn sort_key(filename: &str) -> (String, u8, u32, String) {
    let (sku, secondary_index) = parse_image_variant(filename);
    let variant_rank = if secondary_index.is_some() { 1 } else { 0 };
    let index = secondary_index.unwrap_or(0);
    (sku, variant_rank, index, filename.to_owned())
}

fn parse_image_variant(filename: &str) -> (String, Option<u32>) {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename);

    let (parent, suffix) = match stem.rsplit_once('-') {
        Some(parts) => parts,
        None => return (stem.to_owned(), None),
    };

    if parent.contains('-') && suffix.chars().all(|character| character.is_ascii_digit()) {
        let index = suffix.parse::<u32>().ok();
        (parent.to_owned(), index)
    } else {
        (stem.to_owned(), None)
    }
}

fn secondary_only_summary(image_names: &[String]) -> (usize, Vec<String>) {
    let mut by_sku: BTreeMap<String, SkuImages> = BTreeMap::new();

    for image_name in image_names {
        let (sku, secondary_index) = parse_image_variant(image_name);
        let entry = by_sku.entry(sku).or_default();

        if secondary_index.is_some() {
            entry.secondary_images.push(image_name.clone());
        } else {
            entry.has_main_image = true;
        }
    }

    let mut secondary_only_images = Vec::new();
    let mut secondary_only_sku_count = 0;

    for sku_images in by_sku.into_values() {
        if !sku_images.has_main_image && !sku_images.secondary_images.is_empty() {
            secondary_only_sku_count += 1;
            secondary_only_images.extend(sku_images.secondary_images);
        }
    }

    (secondary_only_sku_count, secondary_only_images)
}

#[derive(Default)]
struct SkuImages {
    has_main_image: bool,
    secondary_images: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{parent_sku_from_filename, secondary_only_summary, sort_image_names};

    #[test]
    fn strips_numeric_image_suffix_when_present() {
        assert_eq!(
            parent_sku_from_filename("HT-033923820684-1.jpg"),
            "HT-033923820684"
        );
    }

    #[test]
    fn does_not_strip_when_no_numeric_suffix() {
        assert_eq!(parent_sku_from_filename("ABC-DEF.jpg"), "ABC-DEF");
    }

    #[test]
    fn keeps_full_stem_when_single_dash_prefix() {
        assert_eq!(parent_sku_from_filename("SKU-1.jpg"), "SKU-1");
    }

    #[test]
    fn sorts_main_image_before_secondary_images() {
        let mut names = vec![
            "HT-033923820684-2.jpg".to_owned(),
            "HT-033923820684.jpg".to_owned(),
            "HT-033923820684-1.jpg".to_owned(),
            "HT-033923820882-1.jpg".to_owned(),
            "HT-033923820882.jpg".to_owned(),
        ];

        sort_image_names(&mut names);

        assert_eq!(
            names,
            vec![
                "HT-033923820684.jpg",
                "HT-033923820684-1.jpg",
                "HT-033923820684-2.jpg",
                "HT-033923820882.jpg",
                "HT-033923820882-1.jpg",
            ]
        );
    }

    #[test]
    fn reports_secondary_only_skus_and_images() {
        let names = vec![
            "HT-000000000001-1.jpg".to_owned(),
            "HT-000000000001-2.jpg".to_owned(),
            "HT-000000000002.jpg".to_owned(),
            "HT-000000000002-1.jpg".to_owned(),
            "HT-000000000003-1.jpg".to_owned(),
        ];

        let (sku_count, images) = secondary_only_summary(&names);

        assert_eq!(sku_count, 2);
        assert_eq!(
            images,
            vec![
                "HT-000000000001-1.jpg",
                "HT-000000000001-2.jpg",
                "HT-000000000003-1.jpg",
            ]
        );
    }
}
