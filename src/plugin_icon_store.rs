//! Thumbnail identity is shared across formats; audio state keeps its own identity.
use crate::{config::PluginKey, state_store};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn hash(parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_le_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

pub(super) fn identity(key: &PluginKey) -> String {
    let name = key.name.trim().to_lowercase();
    let vendor = key.vendor.trim().to_lowercase();
    if name.is_empty() || vendor.is_empty() {
        format!("individual-{}", hash(&[&key.format, &key.id]))
    } else {
        format!("shared-{}", hash(&[&name, &vendor]))
    }
}

fn directory(config: &Path) -> Result<PathBuf, String> {
    Ok(config
        .parent()
        .ok_or("config path has no parent")?
        .join("plugin-icons"))
}

fn path(config: &Path, key: &PluginKey) -> Result<PathBuf, String> {
    Ok(directory(config)?.join(format!("{}.png", identity(key))))
}

fn legacy_path(config: &Path, key: &PluginKey) -> Result<PathBuf, String> {
    let mut hash = Sha256::new();
    hash.update((key.format.len() as u64).to_le_bytes());
    hash.update(key.format.as_bytes());
    hash.update(key.id.as_bytes());
    Ok(directory(config)?.join(format!("{:x}.png", hash.finalize())))
}

fn read(path: &Path) -> Result<Option<image::RgbaImage>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(192);
    limits.max_image_height = Some(128);
    reader.limits(limits);
    reader
        .decode()
        .map(|image| {
            let image = image.into_rgba8();
            (!is_blank(&image)).then_some(image)
        })
        .map_err(|error| error.to_string())
}

// Loading screens can contain a thin border, so exact pixel equality is insufficient.
pub(super) fn is_blank(image: &image::RgbaImage) -> bool {
    let count = u64::from(image.width()) * u64::from(image.height());
    count == 0
        || image
            .pixels()
            .filter(|pixel| pixel.0[..3].iter().all(|&v| v >= 245))
            .count() as u64
            * 100
            >= count * 98
}

pub(super) fn load(
    config: &Path,
    key: &PluginKey,
    aliases: &[PluginKey],
    bounds: (u32, u32),
) -> Result<Option<image::RgbaImage>, String> {
    if let Some(image) = read(&path(config, key)?)? {
        let thumbnail = fit(&image, bounds);
        if thumbnail.dimensions() != image.dimensions() {
            save(config, key, &image, bounds)?;
        }
        return Ok(Some(thumbnail));
    }
    // Existing format/ID images carry no metadata: resolve them only through known keys.
    // Sort candidates so the chosen image does not depend on which row is drawn first.
    let mut candidates: Vec<_> = aliases
        .iter()
        .chain(std::iter::once(key))
        .filter(|candidate| identity(candidate) == identity(key))
        .collect();
    candidates.sort_by(|a, b| (&a.format, &a.id).cmp(&(&b.format, &b.id)));
    candidates.dedup_by(|a, b| a.format == b.format && a.id == b.id);
    for candidate in candidates {
        if let Some(image) = read(&legacy_path(config, candidate)?)? {
            return save(config, key, &image, bounds).map(Some);
        }
    }
    Ok(None)
}

fn fit(image: &image::RgbaImage, bounds: (u32, u32)) -> image::RgbaImage {
    let width = bounds.0.max(1).min(image.width());
    let height = bounds.1.max(1).min(image.height());
    if (width, height) == image.dimensions() {
        return image.clone();
    }
    let scale = (f64::from(width) / f64::from(image.width()))
        .min(f64::from(height) / f64::from(image.height()));
    let width = (f64::from(image.width()) * scale).floor().max(1.0) as u32;
    let height = (f64::from(image.height()) * scale).floor().max(1.0) as u32;
    image::imageops::thumbnail(image, width, height)
}

pub(super) fn save(
    config: &Path,
    key: &PluginKey,
    image: &image::RgbaImage,
    bounds: (u32, u32),
) -> Result<image::RgbaImage, String> {
    if is_blank(image) {
        return Err("Plugin UI is still blank".into());
    }
    let image = fit(image, bounds);
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    state_store::atomic_write(&path(config, key)?, bytes.get_ref())?;
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(format: &str) -> PluginKey {
        PluginKey {
            format: format.into(),
            id: format!("synth-{format}"),
            name: "Synth".into(),
            vendor: "Vendor".into(),
            bundle_path: "X:/plugins/synth.clap".into(),
        }
    }

    #[test]
    fn thumbnails_fit_wide_tall_and_small_images_without_upscaling() {
        for (source, expected) in [
            ((192, 96), (27, 13)),
            ((64, 128), (9, 18)),
            ((8, 4), (8, 4)),
        ] {
            let image =
                image::RgbaImage::from_pixel(source.0, source.1, image::Rgba([30, 60, 90, 255]));
            assert_eq!(fit(&image, (27, 18)).dimensions(), expected);
        }
    }

    #[test]
    fn formats_share_icons_but_other_vendors_names_and_unknown_metadata_do_not() {
        let clap = key("CLAP");
        let mut vst = key("VST3");
        vst.name = " SYNTH ".into();
        vst.vendor = "vendor".into();
        assert_eq!(identity(&clap), identity(&vst));
        vst.vendor = "Other".into();
        assert_ne!(identity(&clap), identity(&vst));
        vst.vendor = clap.vendor;
        vst.name = "Synth Effects".into();
        assert_ne!(identity(&key("CLAP")), identity(&vst));
        let mut unknown = key("CLAP");
        unknown.vendor.clear();
        vst.vendor.clear();
        vst.name = unknown.name.clone();
        assert_ne!(identity(&unknown), identity(&vst));
    }

    #[test]
    fn white_loading_screen_with_border_is_ignored_and_replaced() {
        let directory = std::env::temp_dir().join(format!(
            "cat-icons-blank-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let config = directory.join("config.toml");
        let key = key("CLAP");
        let blank = image::RgbaImage::from_fn(192, 115, |x, _| {
            if x == 0 {
                image::Rgba([220, 220, 220, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            }
        });
        assert!(is_blank(&blank));
        assert!(save(&config, &key, &blank, (27, 18)).is_err());
        std::fs::create_dir_all(directory.join("plugin-icons")).unwrap();
        blank.save(path(&config, &key).unwrap()).unwrap();
        blank.save(legacy_path(&config, &key).unwrap()).unwrap();
        assert!(load(&config, &key, &[], (27, 18)).unwrap().is_none());
        let ready = image::RgbaImage::from_fn(192, 115, |x, _| {
            if x < 15 {
                image::Rgba([30, 30, 30, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            }
        });
        assert!(!is_blank(&ready));
        let saved = save(&config, &key, &ready, (27, 18)).unwrap();
        assert_eq!(saved.dimensions(), (27, 16));
        assert_eq!(
            image::image_dimensions(path(&config, &key).unwrap()).unwrap(),
            (27, 16)
        );
        assert_eq!(
            load(&config, &key, &[], (27, 18)).unwrap().unwrap(),
            fit(&ready, (27, 18))
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn legacy_vst_image_migrates_for_clap_and_survives_without_catalog() {
        let directory = std::env::temp_dir().join(format!(
            "cat-icons-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let config = directory.join("config.toml");
        let clap = key("CLAP");
        let vst = key("VST3");
        assert!(load(&config, &clap, &[], (27, 18)).unwrap().is_none());
        let image =
            image::RgbaImage::from_fn(192, 96, |x, y| image::Rgba([x as u8, y as u8, 50, 255]));
        std::fs::create_dir_all(directory.join("plugin-icons")).unwrap();
        image.save(legacy_path(&config, &vst).unwrap()).unwrap();
        // Shared images from previous versions are also rewritten on first load.
        image.save(path(&config, &clap).unwrap()).unwrap();
        assert_eq!(
            load(&config, &clap, &[], (27, 18))
                .unwrap()
                .unwrap()
                .dimensions(),
            (27, 13)
        );
        assert_eq!(
            image::image_dimensions(path(&config, &clap).unwrap()).unwrap(),
            (27, 13)
        );
        std::fs::remove_file(path(&config, &clap).unwrap()).unwrap();
        let mut other = clap.clone();
        other.vendor = "Other".into();
        assert!(load(&config, &other, std::slice::from_ref(&vst), (27, 18))
            .unwrap()
            .is_none());
        assert_eq!(
            load(&config, &clap, std::slice::from_ref(&vst), (27, 18))
                .unwrap()
                .unwrap(),
            fit(&image, (27, 18))
        );
        std::fs::remove_file(legacy_path(&config, &vst).unwrap()).unwrap();
        assert_eq!(
            load(&config, &clap, &[], (27, 18)).unwrap().unwrap(),
            fit(&image, (27, 18))
        );
        assert_eq!(
            load(&config, &vst, &[], (27, 18)).unwrap().unwrap(),
            fit(&image, (27, 18))
        );
        std::fs::write(path(&config, &clap).unwrap(), b"broken PNG").unwrap();
        assert!(load(&config, &clap, &[], (27, 18)).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
