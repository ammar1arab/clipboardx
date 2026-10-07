use crate::error::AppError;
use image::{DynamicImage, ImageFormat, ImageReader};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    io::Cursor,
    path::{Path, PathBuf},
};

pub struct StoredImage {
    pub hash: String,
    pub path: String,
    pub thumbnail: String,
    pub width: u32,
    pub height: u32,
}

pub struct ImageStore {
    root: PathBuf,
    pub operation: tauri::async_runtime::Mutex<()>,
}

fn failure() -> AppError {
    AppError::storage("Could not access clipboard image storage")
}

fn decode(bytes: &[u8]) -> Result<DynamicImage, AppError> {
    if bytes.is_empty() || bytes.len() > 64 * 1024 * 1024 {
        return Err("Clipboard image exceeds the 64 MiB limit".into());
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| AppError::from("Unknown image format"))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let dimensions = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| AppError::from("Unknown image format"))?
        .into_dimensions()
        .map_err(|_| AppError::from("Invalid image"))?;
    if u64::from(dimensions.0) * u64::from(dimensions.1) > 16 * 1024 * 1024 {
        return Err("Clipboard image exceeds the 16 megapixel limit".into());
    }
    reader
        .decode()
        .map_err(|_| AppError::from("Image cannot be decoded within capture limits"))
}

impl ImageStore {
    pub fn open(root: PathBuf) -> Result<Self, AppError> {
        if root.exists()
            && std::fs::symlink_metadata(&root)
                .map_err(|_| failure())?
                .file_type()
                .is_symlink()
        {
            return Err(failure());
        }
        std::fs::create_dir_all(&root).map_err(|_| failure())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| failure())?;
        }
        Ok(Self {
            root: root.canonicalize().map_err(|_| failure())?,
            operation: tauri::async_runtime::Mutex::new(()),
        })
    }

    fn validate(&self, path: &Path) -> Result<(), AppError> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(failure)?;
        let hash = name
            .strip_suffix("-thumb.png")
            .or_else(|| name.strip_suffix(".png"))
            .ok_or_else(failure)?;
        if path.parent() != Some(self.root.as_path())
            || hash.len() != 64
            || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(failure());
        }
        if path.exists()
            && std::fs::symlink_metadata(path)
                .map_err(|_| failure())?
                .file_type()
                .is_symlink()
        {
            return Err(failure());
        }
        Ok(())
    }

    fn save(&self, image: &DynamicImage, path: &Path) -> Result<(), AppError> {
        self.validate(path)?;
        if path.exists() {
            return Ok(());
        }
        let temporary = self.root.join(format!("{}.tmp", uuid::Uuid::new_v4()));
        let result = image
            .save_with_format(&temporary, ImageFormat::Png)
            .map_err(|_| failure())
            .and_then(|_| std::fs::rename(&temporary, path).map_err(|_| failure()));
        if result.is_err() {
            let _ = std::fs::remove_file(temporary);
        }
        result
    }

    pub fn store(&self, bytes: &[u8]) -> Result<StoredImage, AppError> {
        let image = decode(bytes)?;
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let mut digest = Sha256::new();
        digest.update(width.to_le_bytes());
        digest.update(height.to_le_bytes());
        digest.update(rgba.as_raw());
        let hash = hex::encode(digest.finalize());
        let path = self.root.join(format!("{hash}.png"));
        let thumbnail = self.root.join(format!("{hash}-thumb.png"));
        self.save(&DynamicImage::ImageRgba8(rgba), &path)?;
        self.save(&image.thumbnail(224, 144), &thumbnail)?;
        Ok(StoredImage {
            hash,
            path: path.to_string_lossy().into_owned(),
            thumbnail: thumbnail.to_string_lossy().into_owned(),
            width,
            height,
        })
    }

    pub fn read(&self, path: &str) -> Result<arboard::ImageData<'static>, AppError> {
        let path = Path::new(path);
        self.validate(path)?;
        let metadata = std::fs::metadata(path).map_err(|_| failure())?;
        if metadata.len() > 64 * 1024 * 1024 {
            return Err(failure());
        }
        let image = decode(&std::fs::read(path).map_err(|_| failure())?)?.to_rgba8();
        Ok(arboard::ImageData {
            width: image.width() as usize,
            height: image.height() as usize,
            bytes: std::borrow::Cow::Owned(image.into_raw()),
        })
    }

    pub fn cleanup(&self, referenced: &HashSet<String>) -> Result<(), AppError> {
        for entry in std::fs::read_dir(&self.root).map_err(|_| failure())? {
            let entry = entry.map_err(|_| failure())?;
            let path = entry.path();
            if referenced.contains(&path.to_string_lossy().into_owned()) {
                continue;
            }
            let temporary = path.extension().is_some_and(|extension| extension == "tmp")
                && path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| uuid::Uuid::parse_str(name).is_ok());
            if temporary || self.validate(&path).is_ok() {
                std::fs::remove_file(path).map_err(|_| failure())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_images_deduplicate_and_cleanup_without_arbitrary_paths() {
        let root = std::env::temp_dir().join(format!("clipboardx-images-{}", uuid::Uuid::new_v4()));
        let store = ImageStore::open(root.clone()).unwrap();
        let image = DynamicImage::new_rgba8(800, 600);
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();
        let first = store.store(bytes.get_ref()).unwrap();
        let second = store.store(bytes.get_ref()).unwrap();
        assert_eq!(first.hash, second.hash);
        let thumbnail = image::open(&first.thumbnail).unwrap();
        assert!(thumbnail.width() <= 224 && thumbnail.height() <= 144);
        assert_eq!(store.read(&first.path).unwrap().width, 800);
        assert!(store.read("/etc/passwd").is_err());
        assert!(store.store(b"invalid image").is_err());
        let referenced = HashSet::from([first.path.clone(), first.thumbnail.clone()]);
        store.cleanup(&referenced).unwrap();
        assert!(Path::new(&first.path).exists());
        store.cleanup(&HashSet::new()).unwrap();
        assert!(!Path::new(&first.path).exists());
        std::fs::remove_dir(root).unwrap();
    }
}
