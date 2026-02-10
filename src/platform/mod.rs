pub mod macos;
pub mod windows;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    MacOS,
    Windows,
    Other,
}

pub fn current_platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::MacOS
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Other
    }
}
