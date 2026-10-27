//! `azk reindex`: rebuilds the index from the Markdown notes. See `specs/reindex.md`.

use crate::model::Note;
use crate::nodes::StoreError;
use crate::nodes::embed::Embed;
use crate::nodes::index_upsert::{self, IndexUpsert};
use crate::nodes::reindex::{
    DetectChange, DetectOutcome, Finish, HasQueue, List, Next, NextOutcome, PruneOrphans,
    ReplaceLinks,
};
use crate::utils::{Embedder, Index, NoteStore, Out, System};
use crate::{Node, Transition, Walk, impl_has};
use std::collections::VecDeque;
use std::process::ExitCode;

/// `reindex` takes no arguments, so it has no `usage` and cannot be invoked badly.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub error: Option<String>,
    pub all_ids: Vec<String>,
    pub remaining_ids: VecDeque<String>,
    pub id: Option<String>,
    pub note: Option<Note>,
    pub text_to_embed: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub indexed: usize,
    pub updated: usize,
    pub total: usize,
    pub removed: usize,
}

impl_has!(State: error, id, note, embedding);

impl HasQueue for State {
    fn seed(&mut self, ids: Vec<String>) {
        self.total = ids.len();
        self.remaining_ids = ids.iter().cloned().collect();
        self.all_ids = ids;
        self.indexed = 0;
        self.updated = 0;
    }
    fn all_ids(&self) -> &[String] {
        &self.all_ids
    }
    fn take_next(&mut self) -> Option<String> {
        self.remaining_ids.pop_front()
    }
    fn count_indexed(&mut self) {
        self.indexed += 1;
    }
    fn count_updated(&mut self) {
        self.updated += 1;
    }
    fn set_removed(&mut self, removed: usize) {
        self.removed = removed;
    }
    fn summary(&self) -> (usize, usize, usize, usize) {
        (self.indexed, self.updated, self.removed, self.total)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    List,
    Next,
    DetectChange,
    Embed,
    IndexUpsert,
    ReplaceLinks,
    PruneOrphans,
    Finish,
}

pub struct Reindex<U = System> {
    list: List<U>,
    next: Next,
    detect_change: DetectChange<U>,
    embed: Embed<U>,
    index_upsert: IndexUpsert<U>,
    replace_links: ReplaceLinks<U>,
    prune_orphans: PruneOrphans<U>,
    finish: Finish<U>,
}

impl Reindex {
    #[must_use]
    pub fn new() -> Self {
        Self::with_utils(System)
    }
}

impl Default for Reindex {
    fn default() -> Self {
        Self::new()
    }
}

impl<U: Clone> Reindex<U> {
    #[must_use]
    pub fn with_utils(utils: U) -> Self {
        Self {
            list: List::with_utils(utils.clone()),
            next: Next,
            detect_change: DetectChange::with_utils(utils.clone()),
            embed: Embed::with_utils(utils.clone()),
            index_upsert: IndexUpsert::with_utils(utils.clone()),
            replace_links: ReplaceLinks::with_utils(utils.clone()),
            prune_orphans: PruneOrphans::with_utils(utils.clone()),
            finish: Finish::with_utils(utils),
        }
    }
}

impl<U> Walk for Reindex<U>
where
    U: Embedder + Index + NoteStore + Out + Sync,
{
    type Id = Id;
    type State = State;
    type Error = StoreError;

    async fn step(&self, id: Id, state: &mut State) -> Result<Transition<Id>, StoreError> {
        Ok(match id {
            Id::List => {
                self.list.run(state).await?;
                Transition::Next(Id::Next)
            }
            Id::Next => {
                let Ok(outcome) = self.next.run(state).await;
                match outcome {
                    NextOutcome::Note => Transition::Next(Id::DetectChange),
                    NextOutcome::Done => Transition::Next(Id::PruneOrphans),
                }
            }
            Id::DetectChange => match self.detect_change.run(state).await? {
                DetectOutcome::Read => Transition::Next(Id::Embed),
                DetectOutcome::Vanished => Transition::Next(Id::Next),
            },
            Id::Embed => {
                let Ok(()) = self.embed.run(state).await;
                Transition::Next(Id::IndexUpsert)
            }
            Id::IndexUpsert => {
                let Ok(outcome) = self.index_upsert.run(state).await;
                match outcome {
                    index_upsert::Outcome::Indexed => Transition::Next(Id::ReplaceLinks),
                    index_upsert::Outcome::Failed => Transition::Halt,
                }
            }
            Id::ReplaceLinks => {
                self.replace_links.run(state).await?;
                Transition::Next(Id::Next)
            }
            Id::PruneOrphans => {
                self.prune_orphans.run(state).await?;
                Transition::Next(Id::Finish)
            }
            Id::Finish => {
                let Ok(()) = self.finish.run(state).await;
                Transition::Halt
            }
        })
    }
}

pub async fn run(_args: Vec<String>) -> ExitCode {
    let mut state = State::default();
    if let Err(e) = Reindex::new().run(Id::List, &mut state).await {
        println!("{}", serde_json::json!({ "error": e.to_string() }));
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GraphLink, NoteLink};
    use crate::testing::{Fake, note};
    use serde_json::json;

    async fn reindex(fake: &Fake) -> State {
        let mut state = State::default();
        Reindex::with_utils(fake)
            .run(Id::List, &mut state)
            .await
            .unwrap();
        state
    }

    #[tokio::test]
    async fn indexes_new_notes_embedding_each_body_and_rebuilding_links() {
        let mut a = note("a", "A", "body a");
        a.links.push(NoteLink {
            to: "b".into(),
            relation: "r".into(),
        });
        let fake = Fake {
            vector: Some(vec![1.0]),
            ..Fake::default()
        }
        .with_note(a)
        .with_note(note("b", "B", "body b"));

        reindex(&fake).await;

        assert_eq!(
            fake.printed_json(),
            json!({"indexed":2,"updated":0,"removed":0,"total":2})
        );
        assert_eq!(*fake.embedded.lock().unwrap(), ["body a", "body b"]);
        assert_eq!(
            *fake.links.lock().unwrap(),
            [GraphLink {
                from_id: "a".into(),
                to_id: "b".into(),
                relation: "r".into()
            }]
        );
    }

    #[tokio::test]
    async fn an_unchanged_note_is_not_re_embedded_nor_loses_its_vector_but_links_are_rebuilt() {
        let fake = Fake {
            vector: Some(vec![1.0]),
            ..Fake::default()
        }
        .with_note(note("a", "A", "same"))
        .with_note(note("b", "B", "will change"));
        reindex(&fake).await;
        fake.embedded.lock().unwrap().clear();
        fake.printed.lock().unwrap().clear();
        let mut edited = note("b", "B", "changed");
        edited.links.push(NoteLink {
            to: "a".into(),
            relation: "r".into(),
        });
        fake.write_for_test(edited);

        reindex(&fake).await;

        assert_eq!(
            fake.printed_json(),
            json!({"indexed":0,"updated":1,"removed":0,"total":2})
        );
        assert_eq!(
            *fake.embedded.lock().unwrap(),
            ["changed"],
            "one note's body must not leak into the next"
        );
        assert_eq!(fake.stored_vector("a"), Some(vec![1.0]));
        assert_eq!(fake.links.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn without_an_embeddings_host_it_still_rebuilds_the_index() {
        let fake = Fake::default().with_note(note("a", "A", "x"));

        reindex(&fake).await;

        assert!(fake.indexed.lock().unwrap().contains_key("a"));
        assert_eq!(fake.stored_vector("a"), None);
    }

    #[tokio::test]
    async fn prunes_entries_whose_file_is_gone() {
        let fake = Fake::default()
            .with_note(note("a", "A", "x"))
            .with_note(note("b", "B", "y"));
        reindex(&fake).await;
        fake.files.lock().unwrap().remove("b");
        fake.printed.lock().unwrap().clear();

        reindex(&fake).await;

        assert_eq!(fake.printed_json()["removed"], 1);
        assert!(!fake.indexed.lock().unwrap().contains_key("b"));
    }

    #[tokio::test]
    async fn an_empty_notes_directory_is_fine() {
        let fake = Fake::default();

        reindex(&fake).await;

        assert_eq!(
            fake.printed_json(),
            json!({"indexed":0,"updated":0,"removed":0,"total":0})
        );
    }

    #[tokio::test]
    async fn an_index_failure_abandons_the_queue_with_an_error_value() {
        let fake = Fake {
            fail_index: true,
            ..Fake::default()
        }
        .with_note(note("a", "A", "x"))
        .with_note(note("b", "B", "y"));

        let state = reindex(&fake).await;

        assert_eq!(state.error.as_deref(), Some("index is broken"));
        assert_eq!(fake.printed_json(), json!({"error":"index is broken"}));
        assert_eq!(
            state.remaining_ids.len(),
            1,
            "the rest of the queue is abandoned"
        );
    }
}
