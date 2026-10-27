use crate::Node;
use crate::nodes::HasUrls;
use crate::nodes::read_m3u_file::KHINSIDER_PREFIX;
use crate::utils;
use futures::future::try_join_all;
use std::future::Future;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not resolve a URL")]
    Resolve(#[from] utils::Error),
}

pub trait Utils {
    /// Turns a khinsider song page URL into the direct link of its audio file.
    fn resolve_khinsider(
        &self,
        url: &str,
    ) -> impl Future<Output = Result<String, utils::Error>> + Send;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemUtils;

impl Utils for SystemUtils {
    fn resolve_khinsider(
        &self,
        url: &str,
    ) -> impl Future<Output = Result<String, utils::Error>> + Send {
        utils::khinsider::resolve(url)
    }
}

/// Replaces every khinsider URL with its direct download link, all at once. Other URLs are kept.
#[derive(Clone, Debug, Default)]
pub struct ResolveUrls<U = SystemUtils> {
    utils: U,
}

impl ResolveUrls {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<U> ResolveUrls<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for ResolveUrls<U>
where
    S: HasUrls + Send,
    U: Utils + Sync,
{
    type Outcome = ();
    type Error = Error;

    async fn run(&self, state: &mut S) -> Result<(), Error> {
        let urls = state.urls().to_vec();
        let resolved = try_join_all(urls.iter().map(|url| async move {
            if url.starts_with(KHINSIDER_PREFIX) {
                self.utils.resolve_khinsider(url).await
            } else {
                Ok(url.clone())
            }
        }))
        .await?;
        *state.urls_mut() = resolved;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockState {
        urls: Vec<String>,
    }

    impl HasUrls for MockState {
        fn urls(&self) -> &[String] {
            &self.urls
        }
        fn urls_mut(&mut self) -> &mut Vec<String> {
            &mut self.urls
        }
    }

    /// Resolves a page to `<page>.mp3`, or fails for pages ending in `bad`.
    struct MockUtils;

    impl Utils for MockUtils {
        async fn resolve_khinsider(&self, url: &str) -> Result<String, utils::Error> {
            if url.ends_with("bad") {
                Err(utils::Error::NoDownloadLink(url.to_string()))
            } else {
                Ok(format!("{url}.mp3"))
            }
        }
    }

    fn state(urls: &[&str]) -> MockState {
        MockState {
            urls: urls.iter().map(|u| (*u).to_string()).collect(),
        }
    }

    #[tokio::test]
    async fn resolves_khinsider_urls_and_keeps_the_rest_in_order() {
        let page = format!("{KHINSIDER_PREFIX}/a");
        let mut state = state(&["http://x/1.mp3", &page, "http://x/2.mp3"]);

        ResolveUrls::with_utils(MockUtils)
            .run(&mut state)
            .await
            .unwrap();

        assert_eq!(
            state.urls,
            ["http://x/1.mp3", &format!("{page}.mp3"), "http://x/2.mp3"]
        );
    }

    #[tokio::test]
    async fn fails_and_keeps_the_urls_if_one_cannot_be_resolved() {
        let bad = format!("{KHINSIDER_PREFIX}/bad");
        let mut state = state(&[&bad]);

        let result = ResolveUrls::with_utils(MockUtils).run(&mut state).await;

        assert!(matches!(result, Err(Error::Resolve(_))));
        assert_eq!(state.urls, [bad]);
    }
}
