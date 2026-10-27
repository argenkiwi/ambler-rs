//! The nodes only the `search` walk uses.

use crate::Node;
use crate::model::Hit;
use crate::nodes::{HasArgs, HasEmbedding, HasUsage, StoreError, print_json};
use crate::utils::{Index, Out};
use std::collections::HashMap;
use std::convert::Infallible;

pub const DEFAULT_LIMIT: usize = 5;

/// The query, how many results to return, and the two rankings awaiting the blend.
pub trait HasSearch {
    fn query(&self) -> &str;
    fn set_query(&mut self, query: String);
    fn limit(&self) -> usize;
    fn set_limit(&mut self, limit: usize);
    fn keyword_results(&self) -> &[Hit];
    fn set_keyword_results(&mut self, hits: Vec<Hit>);
    fn semantic_results(&self) -> &[Hit];
    fn set_semantic_results(&mut self, hits: Vec<Hit>);
}

/// Where [`Input`] leaves the walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputOutcome {
    Given,
    /// No query; the usage message is queued.
    Missing,
}

/// Reads the query and an optional result limit (5 when unspecified) from the arguments.
#[derive(Clone, Debug, Default)]
pub struct Input;

impl<S> Node<S> for Input
where
    S: HasArgs + HasSearch + HasUsage + Send,
{
    type Outcome = InputOutcome;
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<InputOutcome, Infallible> {
        let Some(query) = state.args().first().cloned() else {
            *state.usage_mut() = Some(r#"Usage: azk search "<query>" [limit]"#.into());
            return Ok(InputOutcome::Missing);
        };
        let limit = state
            .args()
            .get(1)
            .and_then(|l| l.parse().ok())
            .unwrap_or(DEFAULT_LIMIT);
        state.set_query(query);
        state.set_limit(limit);
        Ok(InputOutcome::Given)
    }
}

/// Scores hits (best first) by rank position, so the two rankings can be added.
fn by_position(hits: Vec<Hit>) -> Vec<Hit> {
    hits.into_iter()
        .enumerate()
        .map(|(position, hit)| Hit {
            score: 1.0 / (60.0 + position as f64 + 1.0),
            ..hit
        })
        .collect()
}

/// Runs the full-text search and queues the query for embedding.
///
/// An empty result is not a dead end: semantic search can still surface notes sharing no keywords.
#[derive(Clone, Debug, Default)]
pub struct Keyword<U> {
    utils: U,
}

impl<U> Keyword<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Keyword<U>
where
    S: HasSearch + HasEmbedding + Send,
    U: Index + Sync,
{
    type Outcome = ();
    type Error = StoreError;

    async fn run(&self, state: &mut S) -> Result<(), StoreError> {
        let hits = self
            .utils
            .keyword_search(state.query(), state.limit())
            .map_err(StoreError)?;
        state.set_keyword_results(by_position(hits));
        *state.text_to_embed_mut() = Some(state.query().to_string());
        Ok(())
    }
}

/// Scores every indexed note against the query vector, keeping the closest few.
///
/// Contributes nothing when the query could not be embedded, degrading to keyword-only.
#[derive(Clone, Debug, Default)]
pub struct Semantic<U> {
    utils: U,
}

impl<U> Semantic<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

impl<S, U> Node<S> for Semantic<U>
where
    S: HasSearch + HasEmbedding + Send,
    U: Index + Sync,
{
    type Outcome = ();
    type Error = StoreError;

    async fn run(&self, state: &mut S) -> Result<(), StoreError> {
        let hits = match state.embedding() {
            Some(vector) => self
                .utils
                .semantic_search(vector, state.limit())
                .map_err(StoreError)?,
            None => Vec::new(),
        };
        state.set_semantic_results(by_position(hits));
        Ok(())
    }
}

/// Blends the two rankings, best match first, and prints them. Terminal.
///
/// Semantic scoring is additive: it boosts a note that also matched by keyword and admits one
/// that matched only semantically. An empty result is `[]`, not an error.
#[derive(Clone, Debug, Default)]
pub struct MergeRank<U> {
    utils: U,
}

impl<U> MergeRank<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self { utils }
    }
}

fn blend(keyword: &[Hit], semantic: &[Hit], limit: usize) -> Vec<Hit> {
    let mut merged: Vec<Hit> = Vec::new();
    let mut at: HashMap<&str, usize> = HashMap::new();
    for hit in keyword.iter().chain(semantic) {
        match at.get(hit.id.as_str()) {
            Some(&i) => merged[i].score += hit.score,
            None => {
                at.insert(&hit.id, merged.len());
                merged.push(hit.clone());
            }
        }
    }
    merged.sort_by(|a, b| b.score.total_cmp(&a.score));
    merged.truncate(limit);
    merged
}

impl<S, U> Node<S> for MergeRank<U>
where
    S: HasSearch + Send,
    U: Out + Sync,
{
    type Outcome = ();
    type Error = Infallible;

    async fn run(&self, state: &mut S) -> Result<(), Infallible> {
        let hits = blend(
            state.keyword_results(),
            state.semantic_results(),
            state.limit(),
        );
        print_json(&self.utils, &serde_json::json!(hits));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(id: &str, score: f64) -> Hit {
        Hit {
            id: id.into(),
            title: id.into(),
            tags: Vec::new(),
            created: String::new(),
            score,
        }
    }

    #[test]
    fn semantic_boosts_a_keyword_match_and_admits_new_ones() {
        let keyword = by_position(vec![hit("a", 0.0), hit("b", 0.0)]);
        let semantic = by_position(vec![hit("b", 0.0), hit("c", 0.0)]);

        let ids: Vec<_> = blend(&keyword, &semantic, 5)
            .into_iter()
            .map(|h| h.id)
            .collect();

        assert_eq!(ids, ["b", "a", "c"]);
    }

    #[test]
    fn blend_respects_the_limit() {
        let keyword = by_position(vec![hit("a", 0.0), hit("b", 0.0), hit("c", 0.0)]);

        assert_eq!(blend(&keyword, &[], 2).len(), 2);
        assert!(blend(&[], &[], 2).is_empty());
    }
}
