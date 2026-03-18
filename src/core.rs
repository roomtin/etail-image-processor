use std::path::Path;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct ImageRow {
    pub file_name: String,
    pub sku: String,
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

pub fn collect_image_rows(image_folder: &Path, unc_base: &str) -> Result<Vec<ImageRow>> {
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

    let rows = image_names
        .iter()
        .map(|image_name| ImageRow {
            file_name: format!(r"{}\{}", normalized_unc, image_name),
            sku: parent_sku_from_filename(image_name),
        })
        .collect();

    Ok(rows)
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

    names.sort_unstable();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::parent_sku_from_filename;

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
}
