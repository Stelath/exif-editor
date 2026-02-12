use super::*;

impl ExifEditorWindow {
    pub(super) fn move_carousel(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.state.photos.is_empty() {
            return;
        }

        let len = self.state.photos.len() as isize;
        let current = self.state.active_photo.unwrap_or(0) as isize;
        let mut next = current + delta;

        if next < 0 {
            next = len - 1;
        }
        if next >= len {
            next = 0;
        }

        self.state.select_photo(next as usize, false);
        self.refresh_tag_rows = true;
        self.status = format!(
            "Selected {} ({}/{})",
            self.state.photos[next as usize].filename,
            next + 1,
            len
        );
        cx.notify();
    }

    pub(super) fn browse_files(&mut self, cx: &mut Context<Self>) {
        let maybe_paths = rfd::FileDialog::new()
            .add_filter("Images", IMAGE_EXTENSIONS)
            .pick_files();

        if let Some(paths) = maybe_paths {
            self.import_paths(paths, cx);
        }
    }

    pub(super) fn import_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let before_count = self.state.photos.len();
        let expanded_paths = expand_paths(paths);
        let skipped = self.state.import_paths(expanded_paths.clone());
        let imported = self.state.photos.len().saturating_sub(before_count);

        if imported > 0 {
            if self.state.active_photo.is_none() {
                self.state.select_photo(0, false);
            }
            self.refresh_tag_rows = true;
            self.status = format!(
                "Imported {imported} photo(s). Skipped {} unsupported path(s).",
                skipped.len()
            );
        } else {
            self.status = String::from("No new supported photos were imported.");
        }

        cx.notify();
    }
    pub(super) fn save_active(&mut self, cx: &mut Context<Self>) {
        let Some(photo_index) = self.state.active_photo else {
            self.status = String::from("No active photo selected");
            cx.notify();
            return;
        };

        match self.state.save_photo_changes(photo_index) {
            Ok(()) => {
                self.status = format!("Saved {}", self.state.photos[photo_index].filename);
            }
            Err(err) => {
                self.status = format!("Save failed: {err}");
            }
        }

        cx.notify();
    }

    pub(super) fn export_active(&mut self, cx: &mut Context<Self>) {
        let Some(photo_index) = self.state.active_photo else {
            self.status = String::from("No active photo selected");
            cx.notify();
            return;
        };

        let Some(export_dir) = rfd::FileDialog::new()
            .set_title("Choose export folder")
            .pick_folder()
        else {
            self.status = String::from("Export cancelled");
            cx.notify();
            return;
        };

        let photo = &self.state.photos[photo_index];
        let output_path = unique_export_path(&export_dir, &photo.filename, "_export");

        if let Err(err) = fs::copy(&photo.path, &output_path) {
            self.status = format!("Failed to copy file to export path: {err}");
            cx.notify();
            return;
        }

        if let Err(err) = MetadataEngine::write(&output_path, &photo.metadata) {
            self.status = format!("File copied, but metadata export failed: {err}");
            cx.notify();
            return;
        }

        self.status = format!("Exported {}", output_path.display());
        cx.notify();
    }

    pub(super) fn save_all(&mut self, cx: &mut Context<Self>) {
        match self.state.save_all_dirty() {
            Ok(count) => {
                self.status = format!("Saved {count} photo(s)");
            }
            Err(err) => {
                self.status = format!("Save all failed: {err}");
            }
        }
        cx.notify();
    }

    pub(super) fn export_all(&mut self, cx: &mut Context<Self>) {
        if self.state.photos.is_empty() {
            self.status = String::from("No photos loaded");
            cx.notify();
            return;
        }

        let Some(export_dir) = rfd::FileDialog::new()
            .set_title("Choose export folder")
            .pick_folder()
        else {
            self.status = String::from("Export cancelled");
            cx.notify();
            return;
        };

        let mut ok_count = 0usize;
        let mut fail_count = 0usize;
        for photo in &self.state.photos {
            let output_path = unique_export_path(&export_dir, &photo.filename, "_export");
            if let Err(_err) = fs::copy(&photo.path, &output_path)
                .and_then(|_| {
                    MetadataEngine::write(&output_path, &photo.metadata)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
                })
            {
                fail_count += 1;
            } else {
                ok_count += 1;
            }
        }

        self.status = format!("Exported {ok_count} photo(s), {fail_count} failed");
        cx.notify();
    }

    pub(super) fn clear_all_metadata(&mut self, cx: &mut Context<Self>) {
        if self.state.photos.is_empty() {
            self.status = String::from("No photos loaded");
            cx.notify();
            return;
        }

        let count = self.state.clear_all_metadata();
        self.status = format!("Cleared all metadata from {count} photo(s)");
        self.refresh_tag_rows = true;
        cx.notify();
    }
}
