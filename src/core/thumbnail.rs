use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::models::{PhotoEntry, ThumbnailData};

pub struct ThumbnailCache {
    pub cache_dir: PathBuf,
    pub max_size: usize,
}

impl ThumbnailCache {
    pub fn new(cache_dir: PathBuf, max_size: usize) -> Self {
        Self {
            cache_dir,
            max_size: max_size.max(1),
        }
    }

    pub fn get_or_generate(&self, photo: &PhotoEntry) -> Option<ThumbnailData> {
        fs::create_dir_all(&self.cache_dir).ok()?;

        let cache_path = self.cache_path(photo.id);
        if cache_path.exists() {
            return Self::load_cached(&cache_path);
        }

        let thumbnail = Self::generate_placeholder(photo);
        Self::save_cached(&cache_path, &thumbnail).ok()?;
        self.enforce_cache_size();

        Some(thumbnail)
    }

    fn cache_path(&self, photo_id: u64) -> PathBuf {
        self.cache_dir.join(format!("{photo_id}.thumb"))
    }

    fn load_cached(path: &Path) -> Option<ThumbnailData> {
        let mut bytes = Vec::new();
        let mut file = fs::File::open(path).ok()?;
        file.read_to_end(&mut bytes).ok()?;

        if bytes.len() < 8 {
            return None;
        }

        let width = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let height = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let pixels = bytes[8..].to_vec();

        Some(ThumbnailData {
            width,
            height,
            pixels,
        })
    }

    fn save_cached(path: &Path, thumbnail: &ThumbnailData) -> std::io::Result<()> {
        let mut file = fs::File::create(path)?;
        file.write_all(&thumbnail.width.to_le_bytes())?;
        file.write_all(&thumbnail.height.to_le_bytes())?;
        file.write_all(&thumbnail.pixels)?;
        file.flush()?;
        Ok(())
    }

    fn generate_placeholder(photo: &PhotoEntry) -> ThumbnailData {
        let width = 64;
        let height = 64;

        let mut hasher = DefaultHasher::new();
        photo.filename.hash(&mut hasher);
        photo.id.hash(&mut hasher);
        let seed = hasher.finish();

        let mut pixels = vec![0; (width * height * 4) as usize];

        for y in 0..height {
            for x in 0..width {
                let index = ((y * width + x) * 4) as usize;
                let base = ((seed + (x as u64 * 31) + (y as u64 * 17)) % 255) as u8;

                pixels[index] = base;
                pixels[index + 1] = base.saturating_add((x * 2) as u8);
                pixels[index + 2] = base.saturating_add((y * 2) as u8);
                pixels[index + 3] = 255;
            }
        }

        ThumbnailData {
            width,
            height,
            pixels,
        }
    }

    fn enforce_cache_size(&self) {
        let Ok(entries) = fs::read_dir(&self.cache_dir) else {
            return;
        };

        let mut indexed_entries: Vec<(PathBuf, SystemTime)> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let modified = entry
                    .metadata()
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                Some((path, modified))
            })
            .collect();

        if indexed_entries.len() <= self.max_size {
            return;
        }

        indexed_entries.sort_by_key(|(_, modified)| *modified);

        let remove_count = indexed_entries.len().saturating_sub(self.max_size);
        for (path, _) in indexed_entries.into_iter().take(remove_count) {
            let _ = fs::remove_file(path);
        }
    }
}
