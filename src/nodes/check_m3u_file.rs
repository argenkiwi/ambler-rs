use crate::Node;
use crate::nodes::HasM3uFilePath;
use std::convert::Infallible;
use std::path::{Path, PathBuf};

/// Where [`CheckM3uFile`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The path names an existing `.m3u` file and is now in the state.
    Valid,
    /// No usable path was given; an error was printed.
    Invalid,
}

pub trait Utils {
    /// The first command-line argument, if any.
    fn first_arg(&self) -> Option<String>;
    fn is_file(&self, path: &Path) -> bool;
    fn print(&self, message: &str);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemUtils;

impl Utils for SystemUtils {
    fn first_arg(&self) -> Option<String> {
        std::env::args().nth(1)
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn print(&self, message: &str) {
        println!("{message}");
    }
}

/// Reads the playlist path from the first argument and checks it is an existing `.m3u` file.
#[derive(Clone, Debug, Default)]
pub struct CheckM3uFile<U = SystemUtils> {
    utils: U,
}

impl CheckM3uFile {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<U> CheckM3uFile<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for CheckM3uFile<U>
where
    S: HasM3uFilePath + Send,
    U: Utils + Sync,
{
    type Outcome = Outcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<Outcome, Infallible> {
        let Some(arg) = self.utils.first_arg() else {
            self.utils.print("Error: Please provide an M3U file path.");
            return Ok(Outcome::Invalid);
        };
        let path = PathBuf::from(arg);
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("m3u"))
        {
            self.utils
                .print("Error: The file must have a .m3u extension.");
            return Ok(Outcome::Invalid);
        }
        if !self.utils.is_file(&path) {
            self.utils.print(&format!(
                "Error: {} does not exist or is not a file.",
                path.display()
            ));
            return Ok(Outcome::Invalid);
        }
        *state.m3u_file_path_mut() = path;
        Ok(Outcome::Valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockState {
        path: PathBuf,
    }

    impl HasM3uFilePath for MockState {
        fn m3u_file_path(&self) -> &Path {
            &self.path
        }
        fn m3u_file_path_mut(&mut self) -> &mut PathBuf {
            &mut self.path
        }
    }

    struct MockUtils {
        arg: Option<&'static str>,
        is_file: bool,
    }

    impl Utils for MockUtils {
        fn first_arg(&self) -> Option<String> {
            self.arg.map(str::to_string)
        }
        fn is_file(&self, _path: &Path) -> bool {
            self.is_file
        }
        fn print(&self, _message: &str) {}
    }

    async fn run(arg: Option<&'static str>, is_file: bool) -> (Outcome, PathBuf) {
        let mut state = MockState::default();
        let Ok(outcome) = CheckM3uFile::with_utils(MockUtils { arg, is_file })
            .run(&mut state)
            .await;
        (outcome, state.path)
    }

    #[tokio::test]
    async fn stores_the_path_if_it_is_an_existing_m3u_file() {
        assert_eq!(
            run(Some("a/list.m3u"), true).await,
            (Outcome::Valid, PathBuf::from("a/list.m3u"))
        );
    }

    #[tokio::test]
    async fn is_invalid_without_an_argument() {
        assert_eq!(run(None, true).await, (Outcome::Invalid, PathBuf::new()));
    }

    #[tokio::test]
    async fn is_invalid_without_the_m3u_extension() {
        assert_eq!(
            run(Some("list.txt"), true).await,
            (Outcome::Invalid, PathBuf::new())
        );
    }

    #[tokio::test]
    async fn is_invalid_if_the_file_does_not_exist() {
        assert_eq!(
            run(Some("list.m3u"), false).await,
            (Outcome::Invalid, PathBuf::new())
        );
    }
}
