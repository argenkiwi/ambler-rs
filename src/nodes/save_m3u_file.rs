use crate::Node;
use crate::nodes::{HasM3uFilePath, HasUrls};
use std::io;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not write the M3U file")]
    Io(#[from] io::Error),
}

pub trait Utils {
    fn write(&self, path: &Path, content: &str) -> io::Result<()>;
    fn print(&self, message: &str);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemUtils;

impl Utils for SystemUtils {
    fn write(&self, path: &Path, content: &str) -> io::Result<()> {
        std::fs::write(path, content)
    }

    fn print(&self, message: &str) {
        println!("{message}");
    }
}

/// Overwrites the playlist with the current URLs, one per line, and prints what it saved.
#[derive(Clone, Debug, Default)]
pub struct SaveM3uFile<U = SystemUtils> {
    utils: U,
}

impl SaveM3uFile {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<U> SaveM3uFile<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for SaveM3uFile<U>
where
    S: HasM3uFilePath + HasUrls + Send,
    U: Utils + Sync,
{
    type Outcome = ();
    type Error = Error;

    async fn run(&self, state: &mut S) -> Result<(), Error> {
        let content = state.urls().join("\n");
        self.utils.write(state.m3u_file_path(), &content)?;
        self.utils.print(&format!(
            "Saved {}:\n{content}",
            state.m3u_file_path().display()
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
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

    #[derive(Default)]
    struct MockUtils {
        written: Mutex<Vec<(PathBuf, String)>>,
        printed: Mutex<Vec<String>>,
        fail: bool,
    }

    impl Utils for &MockUtils {
        fn write(&self, path: &Path, content: &str) -> io::Result<()> {
            if self.fail {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            self.written
                .lock()
                .unwrap()
                .push((path.to_path_buf(), content.to_string()));
            Ok(())
        }
        fn print(&self, message: &str) {
            self.printed.lock().unwrap().push(message.to_string());
        }
    }

    fn state() -> MockState {
        MockState {
            path: PathBuf::from("a/list.m3u"),
            urls: vec!["http://x/1.mp3".into(), "http://x/2.mp3".into()],
        }
    }

    #[tokio::test]
    async fn writes_the_urls_one_per_line_and_prints_them() {
        let utils = MockUtils::default();

        SaveM3uFile::with_utils(&utils)
            .run(&mut state())
            .await
            .unwrap();

        assert_eq!(
            *utils.written.lock().unwrap(),
            [(
                PathBuf::from("a/list.m3u"),
                "http://x/1.mp3\nhttp://x/2.mp3".to_string()
            )]
        );
        assert_eq!(
            *utils.printed.lock().unwrap(),
            ["Saved a/list.m3u:\nhttp://x/1.mp3\nhttp://x/2.mp3"]
        );
    }

    #[tokio::test]
    async fn fails_if_the_file_cannot_be_written() {
        let utils = MockUtils {
            fail: true,
            ..MockUtils::default()
        };

        let result = SaveM3uFile::with_utils(&utils).run(&mut state()).await;

        assert!(matches!(result, Err(Error::Io(_))));
    }
}
