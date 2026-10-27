pub mod check_m3u_file;
pub mod confirm;
pub mod count;
pub mod download_files;
pub mod read_m3u_file;
pub mod resolve_urls;
pub mod save_m3u_file;
pub mod start;
pub mod stop;

use std::path::{Path, PathBuf};

/// A state that has a count field.
pub trait HasCount {
    fn count(&self) -> i32;
    fn count_mut(&mut self) -> &mut i32;
}

/// A state that has the path of an M3U playlist.
pub trait HasM3uFilePath {
    fn m3u_file_path(&self) -> &Path;
    fn m3u_file_path_mut(&mut self) -> &mut PathBuf;
}

/// A state that has a list of URLs.
pub trait HasUrls {
    fn urls(&self) -> &[String];
    fn urls_mut(&mut self) -> &mut Vec<String>;
}
