//! Helpers shared by the `Utils` of several nodes.

pub mod http;
pub mod khinsider;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("request failed")]
    Http(#[from] reqwest::Error),
    #[error("could not write the download")]
    Io(#[from] std::io::Error),
    #[error("invalid URL {0}")]
    Url(String),
    #[error("no download link found on {0}")]
    NoDownloadLink(String),
}
