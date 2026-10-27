use crate::Node;
use crate::nodes::HasEmbedding;
use crate::utils::Embedder;
use std::convert::Infallible;

/// Embeds the pending text, if any. Never fails.
///
/// Shared by `search`, `create`, `update` and `reindex`. An unreachable host, or nothing queued,
/// yields no vector — written on every pass so a previous note's vector is never left behind.
/// `INDEX_UPSERT` reads no vector as "leave the stored one alone".
#[derive(Clone, Debug, Default)]
pub struct Embed<U> {
    utils: U,
}

impl<U> Embed<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Embed<U>
where
    S: HasEmbedding + Send,
    U: Embedder + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        let vector = state
            .text_to_embed()
            .as_deref()
            .and_then(|text| self.utils.embed(text));
        *state.embedding_mut() = vector;
        Ok(())
    }
}
