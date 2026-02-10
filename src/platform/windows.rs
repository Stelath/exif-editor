#[derive(Debug)]
pub enum WindowsIntegrationError {
    Unsupported,
}

impl std::fmt::Display for WindowsIntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "Windows integration is not implemented yet"),
        }
    }
}

impl std::error::Error for WindowsIntegrationError {}

pub fn register_file_associations() -> Result<(), WindowsIntegrationError> {
    Err(WindowsIntegrationError::Unsupported)
}
