#[derive(Debug)]
pub enum MacOsIntegrationError {
    Unsupported,
}

impl std::fmt::Display for MacOsIntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "macOS integration is not implemented yet"),
        }
    }
}

impl std::error::Error for MacOsIntegrationError {}

pub fn register_file_associations() -> Result<(), MacOsIntegrationError> {
    Err(MacOsIntegrationError::Unsupported)
}
