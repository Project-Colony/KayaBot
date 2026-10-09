pub mod omdb;
pub mod thetvdb;
pub mod tmdb;
pub mod tvmaze;

use crate::metadata::error::MetadataError;

// reqwest appends the request URL to its error messages. OMDb and TMDB (v3
// keys) put the API key in the query string, and these messages reach the UI,
// so every request error drops the URL before it is formatted.

/// Maps a failed request to a `MetadataError`, without the request URL.
pub(crate) fn request_error(what: &str, err: reqwest::Error) -> MetadataError {
    if err.is_timeout() {
        MetadataError::Network(format!("{what} request timed out."))
    } else {
        MetadataError::Network(format!("{what} request failed: {}", err.without_url()))
    }
}

/// Maps an unreadable response body to a `MetadataError`, without the request URL.
pub(crate) fn parse_error(what: &str, err: reqwest::Error) -> MetadataError {
    MetadataError::InvalidResponse(format!(
        "Failed to parse {what} response: {}",
        err.without_url()
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    /// A local URL that refuses connections, so requests fail fast offline.
    pub(crate) fn closed_local_url() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind a local port");
        let port = listener.local_addr().expect("local address").port();
        drop(listener);
        format!("http://127.0.0.1:{port}")
    }
}
