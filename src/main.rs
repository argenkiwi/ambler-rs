//! The program this branch builds: runs its primary walk.
//!
//! Run with `cargo run -- playlist.m3u`. Additional walks get their own `src/bin/<name>.rs`.

#[tokio::main]
async fn main() -> Result<(), ambler::walks::m3ud::Error> {
    ambler::walks::m3ud::run().await
}
