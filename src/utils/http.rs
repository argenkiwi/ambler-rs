use crate::utils::Error;
use percent_encoding::percent_decode_str;
use std::path::Path;
use std::sync::LazyLock;
use tokio::io::AsyncWriteExt;
use url::Url;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// Fetches `url` and returns the body as text.
pub async fn fetch_text(url: &str) -> Result<String, Error> {
    Ok(CLIENT
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?)
}

/// Downloads `url` to the file at `path`.
pub async fn download_to(url: &str, path: &Path) -> Result<(), Error> {
    let mut response = CLIENT.get(url).send().await?.error_for_status()?;
    let mut file = tokio::fs::File::create(path).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    Ok(())
}

/// The percent-decoded last path segment of `url`, with any directory part dropped.
pub fn file_name(url: &str) -> Result<String, Error> {
    let parsed = Url::parse(url).map_err(|_| Error::Url(url.to_string()))?;
    let segment = parsed
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap_or_default();
    let decoded = percent_decode_str(segment).decode_utf8_lossy();
    Path::new(decoded.as_ref())
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .ok_or_else(|| Error::Url(url.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_file_after_the_decoded_last_path_segment() {
        assert_eq!(
            file_name("https://a.test/x/01%20Theme.mp3?token=1").unwrap(),
            "01 Theme.mp3"
        );
    }

    #[test]
    fn does_not_let_an_encoded_slash_escape_the_folder() {
        assert_eq!(
            file_name("https://a.test/..%2F..%2Fevil.mp3").unwrap(),
            "evil.mp3"
        );
    }

    #[test]
    fn rejects_urls_without_a_file_name() {
        assert!(matches!(file_name("https://a.test/"), Err(Error::Url(_))));
        assert!(matches!(file_name("not a url"), Err(Error::Url(_))));
    }
}
