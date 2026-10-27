use crate::model::{Link, Note};

pub mod count;
pub mod create;
pub mod create_links;
pub mod delete;
pub mod embed;
pub mod exists_check;
pub mod get;
pub mod help;
pub mod id_arg;
pub mod index_upsert;
pub mod link;
pub mod reindex;
pub mod search;
pub mod start;
pub mod stop;
pub mod update;
pub mod usage;
pub mod write_note;

/// A state that has a count field.
pub trait HasCount {
    fn count(&self) -> i32;
    fn count_mut(&mut self) -> &mut i32;
}

/// A failure of the index or notes directory that the walk reports as `{ "error": ... }`.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct StoreError(pub String);

/// The message `USAGE` prints. Its presence is what makes the run exit 1.
pub trait HasUsage {
    fn usage_mut(&mut self) -> &mut Option<String>;
    fn usage(&self) -> &Option<String>;
}

/// The message a failing node already printed.
pub trait HasError {
    fn error_mut(&mut self) -> &mut Option<String>;
}

/// The verb's arguments from the command line, and the verb itself.
pub trait HasArgs {
    fn args(&self) -> &[String];
}

pub trait HasVerb {
    fn verb(&self) -> &str;
}

/// The id of the note being worked on.
pub trait HasId {
    fn id(&self) -> &Option<String>;
    fn id_mut(&mut self) -> &mut Option<String>;
}

pub trait HasNote {
    fn note(&self) -> &Option<Note>;
    fn note_mut(&mut self) -> &mut Option<Note>;
}

/// Text queued for embedding and the vector it produced.
pub trait HasEmbedding {
    fn text_to_embed(&self) -> &Option<String>;
    fn text_to_embed_mut(&mut self) -> &mut Option<String>;
    fn embedding(&self) -> &Option<Vec<f32>>;
    fn embedding_mut(&mut self) -> &mut Option<Vec<f32>>;
}

/// Links to record in the index, pointing out of `from_id`.
pub trait HasLinks {
    fn source_id(&self) -> &Option<String>;
    fn source_id_mut(&mut self) -> &mut Option<String>;
    fn links(&self) -> &[Link];
    fn links_mut(&mut self) -> &mut Vec<Link>;
}

/// The caller's `title`, `body` and `tags`, each absent when not supplied.
pub trait HasFields {
    fn title(&self) -> &Option<String>;
    fn title_mut(&mut self) -> &mut Option<String>;
    fn body(&self) -> &Option<String>;
    fn body_mut(&mut self) -> &mut Option<String>;
    fn tags(&self) -> &Option<Vec<String>>;
    fn tags_mut(&mut self) -> &mut Option<Vec<String>>;
}

/// Implements the state traits for a struct whose fields carry the spec's snake_case names.
///
/// `impl_has!(State: usage, error, id)` — each name is the field and selects the trait.
#[macro_export]
macro_rules! impl_has {
    ($state:ty: $($name:ident),+ $(,)?) => { $( $crate::impl_has!(@ $state, $name); )+ };
    (@ $s:ty, usage) => {
        impl $crate::nodes::HasUsage for $s {
            fn usage_mut(&mut self) -> &mut Option<String> { &mut self.usage }
            fn usage(&self) -> &Option<String> { &self.usage }
        }
    };
    (@ $s:ty, error) => {
        impl $crate::nodes::HasError for $s {
            fn error_mut(&mut self) -> &mut Option<String> { &mut self.error }
        }
    };
    (@ $s:ty, args) => {
        impl $crate::nodes::HasArgs for $s {
            fn args(&self) -> &[String] { &self.args }
        }
    };
    (@ $s:ty, verb) => {
        impl $crate::nodes::HasVerb for $s {
            fn verb(&self) -> &str { &self.verb }
        }
    };
    (@ $s:ty, id) => {
        impl $crate::nodes::HasId for $s {
            fn id(&self) -> &Option<String> { &self.id }
            fn id_mut(&mut self) -> &mut Option<String> { &mut self.id }
        }
    };
    (@ $s:ty, note) => {
        impl $crate::nodes::HasNote for $s {
            fn note(&self) -> &Option<$crate::model::Note> { &self.note }
            fn note_mut(&mut self) -> &mut Option<$crate::model::Note> { &mut self.note }
        }
    };
    (@ $s:ty, embedding) => {
        impl $crate::nodes::HasEmbedding for $s {
            fn text_to_embed(&self) -> &Option<String> { &self.text_to_embed }
            fn text_to_embed_mut(&mut self) -> &mut Option<String> { &mut self.text_to_embed }
            fn embedding(&self) -> &Option<Vec<f32>> { &self.embedding }
            fn embedding_mut(&mut self) -> &mut Option<Vec<f32>> { &mut self.embedding }
        }
    };
    (@ $s:ty, links) => {
        impl $crate::nodes::HasLinks for $s {
            fn source_id(&self) -> &Option<String> { &self.from_id }
            fn source_id_mut(&mut self) -> &mut Option<String> { &mut self.from_id }
            fn links(&self) -> &[$crate::model::Link] { &self.links }
            fn links_mut(&mut self) -> &mut Vec<$crate::model::Link> { &mut self.links }
        }
    };
    (@ $s:ty, fields) => {
        impl $crate::nodes::HasFields for $s {
            fn title(&self) -> &Option<String> { &self.title }
            fn title_mut(&mut self) -> &mut Option<String> { &mut self.title }
            fn body(&self) -> &Option<String> { &self.body }
            fn body_mut(&mut self) -> &mut Option<String> { &mut self.body }
            fn tags(&self) -> &Option<Vec<String>> { &self.tags }
            fn tags_mut(&mut self) -> &mut Option<Vec<String>> { &mut self.tags }
        }
    };
}

/// Prints one JSON value to stdout.
pub(crate) fn print_json(out: &impl crate::utils::Out, value: &serde_json::Value) {
    out.print(&value.to_string());
}

/// Prints `{ "error": message }` and records the message.
pub(crate) fn fail<S: HasError>(out: &impl crate::utils::Out, state: &mut S, message: String) {
    print_json(out, &serde_json::json!({ "error": message }));
    *state.error_mut() = Some(message);
}
