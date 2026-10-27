use crate::Node;
use crate::nodes::{HasM3uFilePath, HasUrls};
use std::io;
use std::path::Path;

/// URLs starting with this need [`resolve_urls`](crate::nodes::resolve_urls) before downloading.
pub const KHINSIDER_PREFIX: &str = "https://downloads.khinsider.com/game-soundtracks";

/// Where [`ReadM3uFile`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// At least one URL is a khinsider page rather than a direct link.
    NeedsResolution,
    /// Every URL can be downloaded as it is.
    Ready,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not read the M3U file")]
    Io(#[from] io::Error),
}

pub trait Utils {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn print(&self, message: &str);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemUtils;

impl Utils for SystemUtils {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn print(&self, message: &str) {
        println!("{message}");
    }
}

/// Reads the URLs from the playlist, skipping blank and `#` comment lines, and prints them.
#[derive(Clone, Debug, Default)]
pub struct ReadM3uFile<U = SystemUtils> {
    utils: U,
}

impl ReadM3uFile {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<U> ReadM3uFile<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for ReadM3uFile<U>
where
    S: HasM3uFilePath + HasUrls + Send,
    U: Utils + Sync,
{
    type Outcome = Outcome;
    type Error = Error;

    async fn run(&self, state: &mut S) -> Result<Outcome, Error> {
        let content = self.utils.read_to_string(state.m3u_file_path())?;
        let urls: Vec<String> = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_string)
            .collect();

        self.utils
            .print(&format!("URLs in the M3U file:\n{}", urls.join("\n")));
        let needs_resolution = urls.iter().any(|url| url.starts_with(KHINSIDER_PREFIX));
        *state.urls_mut() = urls;

        Ok(if needs_resolution {
            Outcome::NeedsResolution
        } else {
            Outcome::Ready
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[derive(Default)]
    struct MockState {
        path: PathBuf,
        urls: Vec<String>,
    }

    impl HasM3uFilePath for MockState {
        fn m3u_file_path(&self) -> &Path {
            &self.path
        }
        fn m3u_file_path_mut(&mut self) -> &mut PathBuf {
            &mut self.path
        }
    }

    impl HasUrls for MockState {
        fn urls(&self) -> &[String] {
            &self.urls
        }
        fn urls_mut(&mut self) -> &mut Vec<String> {
            &mut self.urls
        }
    }

    struct MockUtils {
        content: io::Result<String>,
    }

    impl Utils for MockUtils {
        fn read_to_string(&self, _path: &Path) -> io::Result<String> {
            match &self.content {
                Ok(content) => Ok(content.clone()),
                Err(e) => Err(io::Error::new(e.kind(), "mock")),
            }
        }
        fn print(&self, _message: &str) {}
    }

    async fn run(content: &str) -> (Outcome, Vec<String>) {
        let mut state = MockState::default();
        let outcome = ReadM3uFile::with_utils(MockUtils {
            content: Ok(content.to_string()),
        })
        .run(&mut state)
        .await
        .unwrap();
        (outcome, state.urls)
    }

    #[tokio::test]
    async fn collects_urls_ignoring_blank_and_comment_lines() {
        let (outcome, urls) = run("#EXTM3U\n\nhttp://a/1.mp3\r\n  # note\nhttp://a/2.mp3\n").await;

        assert_eq!(outcome, Outcome::Ready);
        assert_eq!(urls, ["http://a/1.mp3", "http://a/2.mp3"]);
    }

    #[tokio::test]
    async fn needs_resolution_if_any_url_is_a_khinsider_page() {
        let khinsider = format!("{KHINSIDER_PREFIX}/album/x/1.mp3");
        let (outcome, urls) = run(&format!("http://a/1.mp3\n{khinsider}\n")).await;

        assert_eq!(outcome, Outcome::NeedsResolution);
        assert_eq!(urls.len(), 2);
    }

    #[tokio::test]
    async fn fails_if_the_file_cannot_be_read() {
        let mut state = MockState::default();
        let utils = MockUtils {
            content: Err(io::ErrorKind::NotFound.into()),
        };

        let result = ReadM3uFile::with_utils(utils).run(&mut state).await;

        assert!(matches!(result, Err(Error::Io(_))));
    }
}
