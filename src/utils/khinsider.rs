use crate::utils::{Error, http};
use scraper::{Html, Selector};
use url::Url;

/// Resolves a khinsider song page to the direct link of its audio file.
pub async fn resolve(page_url: &str) -> Result<String, Error> {
    let html = http::fetch_text(page_url).await?;
    let base = Url::parse(page_url).map_err(|_| Error::Url(page_url.to_string()))?;
    find_download_link(&html, &base)
        .map(String::from)
        .ok_or_else(|| Error::NoDownloadLink(page_url.to_string()))
}

/// The page's audio source, or failing that its first link to an `.mp3` (then a `.flac`).
///
/// Links back to the page itself are skipped: song pages carry one with a `?theme=` query whose
/// path ends in `.mp3` too.
fn find_download_link(html: &str, base: &Url) -> Option<Url> {
    let document = Html::parse_document(html);
    let sources = Selector::parse("audio[src]").ok()?;
    let anchors = Selector::parse("a[href]").ok()?;
    let links: Vec<Url> = document
        .select(&sources)
        .filter_map(|audio| audio.value().attr("src"))
        .chain(
            document
                .select(&anchors)
                .filter_map(|a| a.value().attr("href")),
        )
        .filter_map(|link| base.join(link).ok())
        .filter(|url| url.path() != base.path())
        .collect();
    ["mp3", "flac"].into_iter().find_map(|ext| {
        links
            .iter()
            .find(|url| url.path().to_lowercase().ends_with(&format!(".{ext}")))
            .cloned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://downloads.khinsider.com/game-soundtracks/album/x/1.mp3").unwrap()
    }

    #[test]
    fn finds_the_first_mp3_link() {
        let html = r#"<a href="/forums">Forums</a>
            <a href="https://cdn.test/a/01%20Theme.mp3">MP3</a>
            <a href="https://cdn.test/a/01 Theme.flac">FLAC</a>"#;

        assert_eq!(
            find_download_link(html, &base()).unwrap().as_str(),
            "https://cdn.test/a/01%20Theme.mp3"
        );
    }

    #[test]
    fn prefers_the_audio_source_and_skips_links_back_to_the_page() {
        let html = r#"<a href="/game-soundtracks/album/x/1.mp3?theme=dark">Dark</a>
            <audio id="audio" controls src="https://cdn.test/a/1.mp3"></audio>
            <a href="https://cdn.test/a/1.flac">FLAC</a>"#;

        assert_eq!(
            find_download_link(html, &base()).unwrap().as_str(),
            "https://cdn.test/a/1.mp3"
        );
        assert!(
            find_download_link(
                r#"<a href="/game-soundtracks/album/x/1.mp3?theme=dark">x</a>"#,
                &base()
            )
            .is_none()
        );
    }

    #[test]
    fn falls_back_to_flac() {
        let html = r#"<a href="https://cdn.test/a/theme.flac">FLAC</a>"#;

        assert_eq!(
            find_download_link(html, &base()).unwrap().as_str(),
            "https://cdn.test/a/theme.flac"
        );
    }

    #[test]
    fn resolves_relative_links_against_the_page() {
        let html = r#"<a href="/files/theme.mp3">MP3</a>"#;

        assert_eq!(
            find_download_link(html, &base()).unwrap().as_str(),
            "https://downloads.khinsider.com/files/theme.mp3"
        );
    }

    #[test]
    fn finds_nothing_without_an_audio_link() {
        assert!(find_download_link(r#"<a href="/forums">x</a>"#, &base()).is_none());
    }
}
