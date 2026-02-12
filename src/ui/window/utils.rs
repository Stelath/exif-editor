use super::*;

impl MapPopupState {
    pub(super) fn osm_url(&self) -> String {
        format!(
            "https://www.openstreetmap.org/?mlat={:.6}&mlon={:.6}#map=14/{:.6}/{:.6}",
            self.latitude, self.longitude, self.latitude, self.longitude
        )
    }

    pub(super) fn static_map_url(&self) -> String {
        let zoom = 14_u32;
        let n = 2_f64.powi(zoom as i32);
        let x = ((self.longitude + 180.0) / 360.0 * n).floor() as u32;
        let lat_rad = self.latitude.to_radians();
        let y = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n).floor() as u32;
        format!("https://tile.openstreetmap.org/{zoom}/{x}/{y}.png")
    }
}

pub(super) fn image_fallback(message: &str) -> AnyElement {
    div()
        .size_full()
        .bg(gpui::rgb(0x1a1a1a))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .text_color(gpui::rgb(0x888888))
        .child(Icon::new(IconName::File).size(px(16.0)))
        .child(div().text_xs().child(message.to_string()))
        .into_any_element()
}

fn looks_like_image(path: &Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };

    let extension = extension.to_string_lossy().to_ascii_lowercase();
    IMAGE_EXTENSIONS.iter().any(|known| *known == extension)
}

pub(super) fn expand_paths(input_paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in input_paths {
        collect_path(&path, &mut files);
    }

    files
}

fn collect_path(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_file() {
        if looks_like_image(path) {
            files.push(path.to_path_buf());
        }
        return;
    }

    if !path.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        collect_path(&entry.path(), files);
    }
}

pub(super) fn unique_export_path(export_dir: &Path, filename: &str, suffix: &str) -> PathBuf {
    let input_path = Path::new(filename);
    let stem = input_path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("photo"));
    let extension = input_path
        .extension()
        .map(|value| value.to_string_lossy().to_string());

    for attempt in 0..1000_usize {
        let candidate_name = if attempt == 0 {
            match &extension {
                Some(ext) => format!("{stem}{suffix}.{ext}"),
                None => format!("{stem}{suffix}"),
            }
        } else {
            match &extension {
                Some(ext) => format!("{stem}{suffix}_{attempt}.{ext}"),
                None => format!("{stem}{suffix}_{attempt}"),
            }
        };

        let candidate = export_dir.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    export_dir.join(format!("{stem}{suffix}_overflow"))
}

pub(super) fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open").arg(url).status()?;
        if status.success() {
            return Ok(());
        }
        return Err(std::io::Error::other(format!(
            "open command failed with status {status}"
        )));
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd").args(["/C", "start", "", url]).status()?;
        if status.success() {
            return Ok(());
        }
        return Err(std::io::Error::other(format!(
            "start command failed with status {status}"
        )));
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("xdg-open").arg(url).status()?;
        if status.success() {
            return Ok(());
        }
        return Err(std::io::Error::other(format!(
            "xdg-open command failed with status {status}"
        )));
    }

    #[allow(unreachable_code)]
    #[allow(unused_variables)]
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "opening URLs is not supported on this platform",
    ))
}

/// Parse an EXIF date/time string "YYYY:MM:DD HH:MM:SS" into 6 components.
pub(super) fn parse_datetime_parts(
    raw: &str,
) -> Option<(String, String, String, String, String, String)> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    // Expected format: "YYYY:MM:DD HH:MM:SS"
    let parts: Vec<&str> = raw.splitn(2, ' ').collect();
    let date_str = parts.first()?;
    let time_str = parts.get(1).unwrap_or(&"00:00:00");

    let date_parts: Vec<&str> = date_str.split(':').collect();
    let time_parts: Vec<&str> = time_str.split(':').collect();

    if date_parts.len() < 3 {
        return None;
    }

    Some((
        date_parts[0].to_string(),
        date_parts[1].to_string(),
        date_parts[2].to_string(),
        time_parts.first().unwrap_or(&"00").to_string(),
        time_parts.get(1).unwrap_or(&"00").to_string(),
        time_parts.get(2).unwrap_or(&"00").to_string(),
    ))
}
