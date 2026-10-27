//! Downloads the audio files listed in an M3U playlist, resolving khinsider pages to direct links.
//!
//! Run with `cargo run --example m3ud -- playlist.m3u`. See `specs/m3ud.md`.

use ambler::nodes::check_m3u_file::{self, CheckM3uFile};
use ambler::nodes::confirm::{self, Confirm};
use ambler::nodes::download_files::{self, DownloadFiles};
use ambler::nodes::read_m3u_file::{self, ReadM3uFile};
use ambler::nodes::resolve_urls::{self, ResolveUrls};
use ambler::nodes::save_m3u_file::{self, SaveM3uFile};
use ambler::nodes::{HasM3uFilePath, HasUrls};
use ambler::{Node, Transition, Walk};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
struct State {
    m3u_file_path: PathBuf,
    urls: Vec<String>,
}

impl HasM3uFilePath for State {
    fn m3u_file_path(&self) -> &Path {
        &self.m3u_file_path
    }
    fn m3u_file_path_mut(&mut self) -> &mut PathBuf {
        &mut self.m3u_file_path
    }
}

impl HasUrls for State {
    fn urls(&self) -> &[String] {
        &self.urls
    }
    fn urls_mut(&mut self) -> &mut Vec<String> {
        &mut self.urls
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    CheckM3uFile,
    ReadM3uFile,
    PromptResolve,
    ResolveUrls,
    SaveM3uFile,
    PromptDownload,
    DownloadFiles,
    SaveLocalM3uFile,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Read(#[from] read_m3u_file::Error),
    #[error(transparent)]
    Confirm(#[from] confirm::Error),
    #[error(transparent)]
    Resolve(#[from] resolve_urls::Error),
    #[error(transparent)]
    Save(#[from] save_m3u_file::Error),
    #[error(transparent)]
    Download(#[from] download_files::Error),
}

/// The two prompts share one `Confirm` utils type, as do the two saves.
struct M3ud<
    A = check_m3u_file::SystemUtils,
    B = read_m3u_file::SystemUtils,
    C = confirm::SystemUtils,
    D = resolve_urls::SystemUtils,
    E = save_m3u_file::SystemUtils,
    F = download_files::SystemUtils,
> {
    check: CheckM3uFile<A>,
    read: ReadM3uFile<B>,
    prompt_resolve: Confirm<C>,
    resolve: ResolveUrls<D>,
    save: SaveM3uFile<E>,
    prompt_download: Confirm<C>,
    download: DownloadFiles<F>,
    save_local: SaveM3uFile<E>,
}

impl M3ud {
    fn new() -> Self {
        Self {
            check: CheckM3uFile::new(),
            read: ReadM3uFile::new(),
            prompt_resolve: Confirm::new("Some URLs require resolution. Proceed? (y/n)"),
            resolve: ResolveUrls::new(),
            save: SaveM3uFile::new(),
            prompt_download: Confirm::new("Proceed with download? (y/n)"),
            download: DownloadFiles::new(),
            save_local: SaveM3uFile::new(),
        }
    }
}

impl<A, B, C, D, E, F> Walk for M3ud<A, B, C, D, E, F>
where
    A: check_m3u_file::Utils + Sync,
    B: read_m3u_file::Utils + Sync,
    C: confirm::Utils + Sync,
    D: resolve_urls::Utils + Sync,
    E: save_m3u_file::Utils + Sync,
    F: download_files::Utils + Sync,
{
    type Id = Id;
    type State = State;
    type Error = Error;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, Error> {
        Ok(match id {
            Id::CheckM3uFile => {
                let Ok(outcome) = self.check.run(state).await;
                match outcome {
                    check_m3u_file::Outcome::Valid => Transition::Next(Id::ReadM3uFile),
                    check_m3u_file::Outcome::Invalid => Transition::Halt,
                }
            }
            Id::ReadM3uFile => match self.read.run(state).await? {
                read_m3u_file::Outcome::NeedsResolution => Transition::Next(Id::PromptResolve),
                read_m3u_file::Outcome::Ready => Transition::Next(Id::PromptDownload),
            },
            Id::PromptResolve => match self.prompt_resolve.run(state).await? {
                confirm::Outcome::Yes => Transition::Next(Id::ResolveUrls),
                confirm::Outcome::No => Transition::Halt,
            },
            Id::ResolveUrls => {
                self.resolve.run(state).await?;
                Transition::Next(Id::SaveM3uFile)
            }
            Id::SaveM3uFile => {
                self.save.run(state).await?;
                Transition::Next(Id::PromptDownload)
            }
            Id::PromptDownload => match self.prompt_download.run(state).await? {
                confirm::Outcome::Yes => Transition::Next(Id::DownloadFiles),
                confirm::Outcome::No => Transition::Halt,
            },
            Id::DownloadFiles => {
                self.download.run(state).await?;
                Transition::Next(Id::SaveLocalM3uFile)
            }
            Id::SaveLocalM3uFile => {
                self.save_local.run(state).await?;
                Transition::Halt
            }
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    M3ud::new()
        .run(Id::CheckM3uFile, &mut State::default())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambler::nodes::read_m3u_file::KHINSIDER_PREFIX;
    use ambler::utils;
    use std::collections::{BTreeMap, VecDeque};
    use std::io;
    use std::sync::Mutex;

    /// Plays back the argument and typed answers, and stands in for the file system and the
    /// network: files are in memory, a khinsider page resolves to `http://cdn.test/<last
    /// segment>.mp3`, and every download is empty.
    struct Script {
        arg: Option<&'static str>,
        answers: Mutex<VecDeque<&'static str>>,
        files: Mutex<BTreeMap<PathBuf, String>>,
        printed: Mutex<Vec<String>>,
    }

    impl Script {
        fn new(arg: Option<&'static str>, content: &str, answers: &[&'static str]) -> Self {
            Self {
                arg,
                answers: Mutex::new(answers.iter().copied().collect()),
                files: Mutex::new(BTreeMap::from([(
                    PathBuf::from("music/list.m3u"),
                    content.to_string(),
                )])),
                printed: Mutex::default(),
            }
        }

        fn record(&self, message: &str) {
            self.printed.lock().unwrap().push(message.to_string());
        }

        fn files(&self) -> Vec<(String, String)> {
            self.files
                .lock()
                .unwrap()
                .iter()
                .map(|(path, content)| (path.display().to_string(), content.clone()))
                .collect()
        }
    }

    impl check_m3u_file::Utils for &Script {
        fn first_arg(&self) -> Option<String> {
            self.arg.map(str::to_string)
        }
        fn is_file(&self, path: &Path) -> bool {
            self.files.lock().unwrap().contains_key(path)
        }
        fn print(&self, message: &str) {
            self.record(message);
        }
    }

    impl read_m3u_file::Utils for &Script {
        fn read_to_string(&self, path: &Path) -> io::Result<String> {
            self.files
                .lock()
                .unwrap()
                .get(path)
                .cloned()
                .ok_or_else(|| io::ErrorKind::NotFound.into())
        }
        fn print(&self, message: &str) {
            self.record(message);
        }
    }

    impl confirm::Utils for &Script {
        fn read_line(&self, _message: &str) -> io::Result<Option<String>> {
            Ok(self.answers.lock().unwrap().pop_front().map(str::to_string))
        }
        fn print(&self, message: &str) {
            self.record(message);
        }
    }

    impl resolve_urls::Utils for &Script {
        async fn resolve_khinsider(&self, url: &str) -> Result<String, utils::Error> {
            Ok(format!(
                "http://cdn.test/{}.mp3",
                url.rsplit('/').next().unwrap()
            ))
        }
    }

    impl save_m3u_file::Utils for &Script {
        fn write(&self, path: &Path, content: &str) -> io::Result<()> {
            self.files
                .lock()
                .unwrap()
                .insert(path.to_path_buf(), content.to_string());
            Ok(())
        }
        fn print(&self, message: &str) {
            self.record(message);
        }
    }

    impl download_files::Utils for &Script {
        fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
            Ok(())
        }
        fn file_name(&self, url: &str) -> Result<String, utils::Error> {
            Ok(url.rsplit('/').next().unwrap().to_string())
        }
        async fn download(&self, _url: &str, path: &Path) -> Result<(), utils::Error> {
            self.files
                .lock()
                .unwrap()
                .insert(path.to_path_buf(), String::new());
            Ok(())
        }
        fn remove_file(&self, path: &Path) -> io::Result<()> {
            self.files.lock().unwrap().remove(path);
            Ok(())
        }
        fn file_uri(&self, path: &Path) -> io::Result<String> {
            Ok(format!("file:///abs/{}", path.display()))
        }
        fn print(&self, message: &str) {
            self.record(message);
        }
    }

    fn m3ud(script: &Script) -> M3ud<&Script, &Script, &Script, &Script, &Script, &Script> {
        M3ud {
            check: CheckM3uFile::with_utils(script),
            read: ReadM3uFile::with_utils(script),
            prompt_resolve: Confirm::with_utils("resolve?", script),
            resolve: ResolveUrls::with_utils(script),
            save: SaveM3uFile::with_utils(script),
            prompt_download: Confirm::with_utils("download?", script),
            download: DownloadFiles::with_utils(script),
            save_local: SaveM3uFile::with_utils(script),
        }
    }

    async fn walk(script: &Script) -> State {
        let mut state = State::default();
        m3ud(script)
            .run(Id::CheckM3uFile, &mut state)
            .await
            .unwrap();
        state
    }

    fn khinsider_playlist() -> String {
        format!("#EXTM3U\n{KHINSIDER_PREFIX}/album/x/one\nhttp://other.test/two.mp3\n")
    }

    #[tokio::test]
    async fn resolves_downloads_and_saves_a_local_playlist() {
        let script = Script::new(
            Some("music/list.m3u"),
            &khinsider_playlist(),
            &["maybe", "y", "yes"],
        );

        let state = walk(&script).await;

        assert_eq!(
            state.m3u_file_path,
            PathBuf::from("music/list/playlist.m3u")
        );
        assert_eq!(
            state.urls,
            [
                "file:///abs/music/list/one.mp3",
                "file:///abs/music/list/two.mp3"
            ]
        );
        assert_eq!(
            script.files(),
            [
                ("music/list/one.mp3".into(), String::new()),
                ("music/list/playlist.m3u".into(), state.urls.join("\n")),
                ("music/list/two.mp3".into(), String::new()),
            ]
        );
        assert!(
            script
                .printed
                .lock()
                .unwrap()
                .contains(&"Please answer y or n.".to_string())
        );
    }

    #[tokio::test]
    async fn saves_the_resolved_playlist_before_asking_to_download() {
        let script = Script::new(Some("music/list.m3u"), &khinsider_playlist(), &["y", "n"]);

        walk(&script).await;

        assert_eq!(
            script.files(),
            [(
                "music/list.m3u".into(),
                "http://cdn.test/one.mp3\nhttp://other.test/two.mp3".into()
            )]
        );
    }

    #[tokio::test]
    async fn skips_resolution_if_no_url_needs_it() {
        let script = Script::new(Some("music/list.m3u"), "http://a.test/1.mp3\n", &["y"]);

        let state = walk(&script).await;

        assert_eq!(state.urls, ["file:///abs/music/list/1.mp3"]);
        assert_eq!(script.files().len(), 2);
    }

    #[tokio::test]
    async fn stops_without_changes_if_resolution_is_declined() {
        let content = khinsider_playlist();
        let script = Script::new(Some("music/list.m3u"), &content, &["n"]);

        walk(&script).await;

        assert_eq!(script.files(), [("music/list.m3u".into(), content)]);
    }

    #[tokio::test]
    async fn stops_without_downloading_if_download_is_declined() {
        let script = Script::new(Some("music/list.m3u"), "http://a.test/1.mp3\n", &["no"]);

        let state = walk(&script).await;

        assert_eq!(state.m3u_file_path, PathBuf::from("music/list.m3u"));
        assert_eq!(script.files().len(), 1);
    }

    #[tokio::test]
    async fn stops_with_an_error_for_a_bad_path() {
        for arg in [None, Some("music/list.txt"), Some("music/missing.m3u")] {
            let script = Script::new(arg, "http://a.test/1.mp3\n", &[]);

            let state = walk(&script).await;

            assert_eq!(state.m3u_file_path, PathBuf::new());
            assert!(script.printed.lock().unwrap()[0].starts_with("Error:"));
        }
    }
}
