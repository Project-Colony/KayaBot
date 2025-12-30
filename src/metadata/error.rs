use std::fmt;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum MetadataError {
    Network(String),
    NotFound(String),
    InvalidResponse(String),
    RateLimited(String),
    Cache(String),
    Other(String),
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(message) => write!(f, "Network error: {message}"),
            Self::NotFound(message) => write!(f, "Not found: {message}"),
            Self::InvalidResponse(message) => write!(f, "Invalid response: {message}"),
            Self::RateLimited(message) => write!(f, "Rate limited: {message}"),
            Self::Cache(message) => write!(f, "Cache error: {message}"),
            Self::Other(message) => write!(f, "Metadata error: {message}"),
        }
    }
}

impl std::error::Error for MetadataError {}
