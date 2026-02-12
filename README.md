<table align="center" border="0" cellspacing="0" cellpadding="0" style="border:none;">
  <tr>
    <td style="border:none; padding-right:12px; vertical-align:middle;">
      <img src="assets/logo.png" alt="Exif Editor Logo" width="64" style="border-radius:22px; display:block;">
    </td>
    <td style="border:none; vertical-align:middle;">
      <h1 style="margin:0;">Exif Editor</h1>
    </td>
  </tr>
</table>

<p align="center">
  A fast, native photo metadata editor for macOS & <s>Windows</s> (Coming Soon) — view, edit, and strip EXIF/IPTC/XMP tags with ease.
</p>

![Exif Editor Screenshot](assets/window-capture.png)

---

## Features

- **View & Edit EXIF Data** — Inspect and modify every metadata tag embedded in your photos, including camera make/model, exposure settings, GPS coordinates, timestamps, and more.
- **Bulk Processing** — Apply stripping presets across hundreds of images at once.
- **GPS Map Preview** — Visualize embedded GPS coordinates on an interactive map.
- **Wide Filetype Support** — Full EXIF parsing for HEIC/HEIF, JPEG, PNG, TIFF, WebP, AVIF, and JXL.
- **Native macOS App** — Built with [GPUI](https://gpui.rs) for a truly native, GPU-accelerated experience.

## Getting Started

### Prerequisites

- **Rust** (1.80+) via [rustup](https://rustup.rs)
- **macOS** 13+ (Ventura or later recommended)

### Build & Run

```bash
# Clone the repository
git clone https://github.com/Stelath/exif-editor.git
cd exif-editor

# Run in development mode
cargo run

# Build a release binary
cargo build --release
```

### Build macOS Application Bundle

```bash
# Install cargo-bundle
cargo install cargo-bundle

# Build the .app bundle
cargo bundle --release
```

The application will be available at `target/release/bundle/osx/Exif Editor.app`.

## Usage

1. **Import Photos** — Drag and drop images into the window or use the file picker.
2. **Inspect Metadata** — Select a photo to view all EXIF, IPTC, and XMP tags in the right panel.
3. **Edit Tags** — Click any editable field to modify its value. Use the date picker for timestamps and the map popup for GPS coordinates.
4. **Add Tags** — Use the "Add Metadata" button to insert new metadata fields.
6. **Save** — Save changes back to the file, or export with a suffix to preserve the original.

## TODO
- [ ] Windows Support
- [ ] Add Presets (eg. a Privacy Preset that will strip any personally identifiable metadata)
- [ ] MacOS Quick Actions in Finder to Strip Metadata without ever needing to open the app

## License

This project is licensed under the [MIT License](LICENSE).
