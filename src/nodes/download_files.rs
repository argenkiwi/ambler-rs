use crate::Node;
use crate::nodes::{HasM3uFilePath, HasUrls};
use crate::utils;
use futures::future::try_join_all;
use std::collections::HashSet;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not prepare the output folder or the M3U file")]
    Io(#[from] io::Error),
    #[error("could not download a file")]
    Download(#[from] utils::Error),
}

pub trait Utils {
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    /// The name a download of `url` would get by default.
    fn file_name(&self, url: &str) -> Result<String, utils::Error>;
    /// Downloads `url` to the file at `path`.
    fn download(
        &self,
        url: &str,
        path: &Path,
    ) -> impl Future<Output = Result<(), utils::Error>> + Send;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    /// The `file://` URI of `path`, made absolute.
    fn file_uri(&self, path: &Path) -> io::Result<String>;
    fn print(&self, message: &str);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemUtils;

impl Utils for SystemUtils {
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn file_name(&self, url: &str) -> Result<String, utils::Error> {
        utils::http::file_name(url)
    }

    fn download(
        &self,
        url: &str,
        path: &Path,
    ) -> impl Future<Output = Result<(), utils::Error>> + Send {
        utils::http::download_to(url, path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn file_uri(&self, path: &Path) -> io::Result<String> {
        let absolute = std::path::absolute(path)?;
        url::Url::from_file_path(&absolute)
            .map(String::from)
            .map_err(|()| io::Error::new(io::ErrorKind::InvalidInput, "not an absolute path"))
    }

    fn print(&self, message: &str) {
        println!("{message}");
    }
}

/// Gives each name in turn a unique spelling: a repeat becomes `stem (2).ext`, `stem (3).ext`, and
/// so on. Names are compared case-insensitively, as common file systems do.
fn unique_names(names: Vec<String>) -> Vec<String> {
    let mut taken = HashSet::new();
    names
        .into_iter()
        .map(|name| {
            let mut candidate = name.clone();
            let mut n = 1;
            while !taken.insert(candidate.to_lowercase()) {
                n += 1;
                candidate = numbered(&name, n);
            }
            candidate
        })
        .collect()
}

fn numbered(name: &str, n: usize) -> String {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    match path.extension() {
        Some(ext) => format!("{stem} ({n}).{}", ext.to_string_lossy()),
        None => format!("{stem} ({n})"),
    }
}

/// Downloads every URL into a folder named after the playlist (beside it, without the extension),
/// all at once, renaming repeated file names so that none overwrites another. Then removes the original playlist and points the state at
/// `<folder>/playlist.m3u`, with the `file://` URIs of the downloads as its URLs.
#[derive(Clone, Debug, Default)]
pub struct DownloadFiles<U = SystemUtils> {
    utils: U,
}

impl DownloadFiles {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<U> DownloadFiles<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for DownloadFiles<U>
where
    S: HasM3uFilePath + HasUrls + Send,
    U: Utils + Sync,
{
    type Outcome = ();
    type Error = Error;

    async fn run(&self, state: &mut S) -> Result<(), Error> {
        let folder = state.m3u_file_path().with_extension("");
        self.utils.create_dir_all(&folder)?;
        self.utils
            .print(&format!("Downloading to {}", folder.display()));

        let urls = state.urls().to_vec();
        let names = unique_names(
            urls.iter()
                .map(|url| self.utils.file_name(url))
                .collect::<Result<_, _>>()?,
        );
        let files: Vec<PathBuf> = names.iter().map(|name| folder.join(name)).collect();
        try_join_all(
            urls.iter()
                .zip(&files)
                .map(|(url, file)| self.utils.download(url, file)),
        )
        .await?;
        let uris = files
            .iter()
            .map(|file| self.utils.file_uri(file))
            .collect::<io::Result<Vec<_>>>()?;

        self.utils.remove_file(state.m3u_file_path())?;
        *state.m3u_file_path_mut() = folder.join("playlist.m3u");
        *state.urls_mut() = uris;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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

    /// Names a download after the last URL segment, and records it unless the URL ends in `bad`.
    #[derive(Default)]
    struct MockUtils {
        downloaded: Mutex<Vec<(String, PathBuf)>>,
        dirs: Mutex<Vec<PathBuf>>,
        removed: Mutex<Vec<PathBuf>>,
    }

    impl Utils for &MockUtils {
        fn create_dir_all(&self, path: &Path) -> io::Result<()> {
            self.dirs.lock().unwrap().push(path.to_path_buf());
            Ok(())
        }
        fn file_name(&self, url: &str) -> Result<String, utils::Error> {
            Ok(url.rsplit('/').next().unwrap().to_string())
        }
        async fn download(&self, url: &str, path: &Path) -> Result<(), utils::Error> {
            if url.ends_with("bad") {
                return Err(utils::Error::Url(url.to_string()));
            }
            self.downloaded
                .lock()
                .unwrap()
                .push((url.to_string(), path.to_path_buf()));
            Ok(())
        }
        fn remove_file(&self, path: &Path) -> io::Result<()> {
            self.removed.lock().unwrap().push(path.to_path_buf());
            Ok(())
        }
        fn file_uri(&self, path: &Path) -> io::Result<String> {
            Ok(format!("file:///abs/{}", path.display()))
        }
        fn print(&self, _message: &str) {}
    }

    fn state(urls: &[&str]) -> MockState {
        MockState {
            path: PathBuf::from("music/list.m3u"),
            urls: urls.iter().map(|u| (*u).to_string()).collect(),
        }
    }

    #[tokio::test]
    async fn downloads_into_a_folder_and_repoints_the_state_at_the_local_playlist() {
        let utils = MockUtils::default();
        let mut state = state(&["http://x/1.mp3", "http://x/2.mp3"]);

        DownloadFiles::with_utils(&utils)
            .run(&mut state)
            .await
            .unwrap();

        assert_eq!(*utils.dirs.lock().unwrap(), [PathBuf::from("music/list")]);
        assert_eq!(
            *utils.removed.lock().unwrap(),
            [PathBuf::from("music/list.m3u")]
        );
        assert_eq!(state.path, PathBuf::from("music/list/playlist.m3u"));
        assert_eq!(
            state.urls,
            [
                "file:///abs/music/list/1.mp3",
                "file:///abs/music/list/2.mp3"
            ]
        );
    }

    #[tokio::test]
    async fn gives_repeated_file_names_a_numbered_suffix() {
        let utils = MockUtils::default();
        let mut state = state(&[
            "http://x/a/1.mp3",
            "http://y/1.mp3",
            "http://z/1.MP3",
            "http://x/2.mp3",
            "http://x/notes",
            "http://y/notes",
        ]);

        DownloadFiles::with_utils(&utils)
            .run(&mut state)
            .await
            .unwrap();

        let mut targets: Vec<_> = utils
            .downloaded
            .lock()
            .unwrap()
            .iter()
            .map(|(_, path)| path.display().to_string())
            .collect();
        targets.sort();
        let mut expected = [
            "music/list/1.mp3",
            "music/list/1 (2).mp3",
            "music/list/1 (3).MP3",
            "music/list/2.mp3",
            "music/list/notes",
            "music/list/notes (2)",
        ]
        .map(String::from);
        expected.sort();
        assert_eq!(targets, expected);
        assert_eq!(state.urls[1], "file:///abs/music/list/1 (2).mp3");
    }

    #[tokio::test]
    async fn keeps_the_playlist_and_state_if_a_download_fails() {
        let utils = MockUtils::default();
        let mut state = state(&["http://x/1.mp3", "http://x/bad"]);

        let result = DownloadFiles::with_utils(&utils).run(&mut state).await;

        assert!(matches!(result, Err(Error::Download(_))));
        assert!(utils.removed.lock().unwrap().is_empty());
        assert_eq!(state.path, PathBuf::from("music/list.m3u"));
        assert_eq!(state.urls.len(), 2);
    }
}
