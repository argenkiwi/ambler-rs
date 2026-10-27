//! `azk search`: ranks notes against a query. See `specs/search.md`.

use crate::model::Hit;
use crate::nodes::StoreError;
use crate::nodes::embed::Embed;
use crate::nodes::search::{
    DEFAULT_LIMIT, HasSearch, Input, InputOutcome, Keyword, MergeRank, Semantic,
};
use crate::nodes::usage::Usage;
use crate::utils::{Embedder, Index, Out, System};
use crate::{Node, Transition, Walk, impl_has};
use std::process::ExitCode;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub args: Vec<String>,
    pub usage: Option<String>,
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub text_to_embed: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub keyword_results: Vec<Hit>,
    pub semantic_results: Vec<Hit>,
}

impl_has!(State: args, usage, embedding);

impl HasSearch for State {
    fn query(&self) -> &str {
        self.query.as_deref().unwrap_or_default()
    }
    fn set_query(&mut self, query: String) {
        self.query = Some(query);
    }
    fn limit(&self) -> usize {
        self.limit.unwrap_or(DEFAULT_LIMIT)
    }
    fn set_limit(&mut self, limit: usize) {
        self.limit = Some(limit);
    }
    fn keyword_results(&self) -> &[Hit] {
        &self.keyword_results
    }
    fn set_keyword_results(&mut self, hits: Vec<Hit>) {
        self.keyword_results = hits;
    }
    fn semantic_results(&self) -> &[Hit] {
        &self.semantic_results
    }
    fn set_semantic_results(&mut self, hits: Vec<Hit>) {
        self.semantic_results = hits;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Input,
    Keyword,
    Embed,
    Semantic,
    MergeRank,
    Usage,
}

pub struct Search<U = System> {
    input: Input,
    keyword: Keyword<U>,
    embed: Embed<U>,
    semantic: Semantic<U>,
    merge_rank: MergeRank<U>,
    usage: Usage<U>,
}

impl Search {
    #[must_use]
    pub fn new() -> Self {
        Self::with_utils(System)
    }
}

impl Default for Search {
    fn default() -> Self {
        Self::new()
    }
}

impl<U: Clone> Search<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self {
            input: Input,
            keyword: Keyword::with_utils(utils.clone()),
            embed: Embed::with_utils(utils.clone()),
            semantic: Semantic::with_utils(utils.clone()),
            merge_rank: MergeRank::with_utils(utils.clone()),
            usage: Usage::with_utils(utils),
        }
    }
}

impl<U> Walk for Search<U>
where
    U: Embedder + Index + Out + Sync,
{
    type Id = Id;
    type State = State;
    type Error = StoreError;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, StoreError> {
        Ok(match id {
            Id::Input => {
                let Ok(outcome) = self.input.run(state).await;
                match outcome {
                    InputOutcome::Given => Transition::Next(Id::Keyword),
                    InputOutcome::Missing => Transition::Next(Id::Usage),
                }
            }
            Id::Keyword => {
                self.keyword.run(state).await?;
                Transition::Next(Id::Embed)
            }
            Id::Embed => {
                let Ok(()) = self.embed.run(state).await;
                Transition::Next(Id::Semantic)
            }
            Id::Semantic => {
                self.semantic.run(state).await?;
                Transition::Next(Id::MergeRank)
            }
            Id::MergeRank => {
                let Ok(()) = self.merge_rank.run(state).await;
                Transition::Halt
            }
            Id::Usage => {
                let Ok(()) = self.usage.run(state).await;
                Transition::Halt
            }
        })
    }
}

pub async fn run(args: Vec<String>) -> ExitCode {
    let mut state = State {
        args,
        ..State::default()
    };
    if let Err(e) = Search::new().run(Id::Input, &mut state).await {
        println!("{}", serde_json::json!({ "error": e.to_string() }));
    }
    super::exit_code(&state.usage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Fake;
    use serde_json::json;

    async fn search(fake: &Fake, args: &[&str]) -> State {
        let mut state = State {
            args: args.iter().map(ToString::to_string).collect(),
            ..State::default()
        };
        Search::with_utils(fake)
            .run(Id::Input, &mut state)
            .await
            .unwrap();
        state
    }

    fn hit(id: &str) -> Hit {
        Hit {
            id: id.into(),
            title: id.to_uppercase(),
            tags: Vec::new(),
            created: "c".into(),
            score: 0.0,
        }
    }

    fn ids(fake: &Fake) -> Vec<String> {
        fake.printed_json()
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["id"].as_str().unwrap().to_string())
            .collect()
    }

    #[tokio::test]
    async fn blends_both_rankings_and_embeds_the_query() {
        let fake = Fake {
            vector: Some(vec![1.0]),
            keyword_hits: vec![hit("a"), hit("b")],
            semantic_hits: vec![hit("b"), hit("c")],
            ..Fake::default()
        };

        let state = search(&fake, &["state machines"]).await;

        assert_eq!(ids(&fake), ["b", "a", "c"]);
        assert_eq!(*fake.embedded.lock().unwrap(), ["state machines"]);
        assert_eq!(state.limit, Some(5));
        assert!(fake.printed_json()[0].get("body").is_none());
    }

    #[tokio::test]
    async fn degrades_to_keyword_only_without_an_embeddings_host() {
        let fake = Fake {
            vector: None,
            keyword_hits: vec![hit("a")],
            semantic_hits: vec![hit("zzz")],
            ..Fake::default()
        };

        search(&fake, &["q"]).await;

        assert_eq!(ids(&fake), ["a"]);
    }

    #[tokio::test]
    async fn semantic_alone_can_surface_a_note() {
        let fake = Fake {
            vector: Some(vec![1.0]),
            semantic_hits: vec![hit("s")],
            ..Fake::default()
        };

        search(&fake, &["q"]).await;

        assert_eq!(ids(&fake), ["s"]);
    }

    #[tokio::test]
    async fn the_limit_argument_caps_results() {
        let fake = Fake {
            keyword_hits: vec![hit("a"), hit("b"), hit("c")],
            ..Fake::default()
        };

        search(&fake, &["q", "2"]).await;

        assert_eq!(ids(&fake), ["a", "b"]);
    }

    #[tokio::test]
    async fn no_results_print_an_empty_array() {
        let fake = Fake::default();

        let state = search(&fake, &["q"]).await;

        assert_eq!(state.usage, None);
        assert_eq!(fake.printed_json(), json!([]));
    }

    #[tokio::test]
    async fn a_missing_query_prints_usage() {
        let fake = Fake::default();

        let state = search(&fake, &[]).await;

        assert!(state.usage.unwrap().starts_with("Usage: azk search"));
        assert!(fake.printed().is_empty());
        assert_eq!(fake.eprinted().len(), 1);
    }
}
