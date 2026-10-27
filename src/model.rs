use serde::{Deserialize, Serialize};

/// A link as stored in a note's own frontmatter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteLink {
    pub to: String,
    pub relation: String,
}

/// A note: one Markdown file with YAML frontmatter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created: String,
    pub updated: String,
    pub links: Vec<NoteLink>,
    pub body: String,
}

/// A link in caller form, as given to `create` and printed back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub to_id: String,
    pub relation: String,
}

impl From<&NoteLink> for Link {
    fn from(link: &NoteLink) -> Self {
        Self {
            to_id: link.to.clone(),
            relation: link.relation.clone(),
        }
    }
}

impl From<&Link> for NoteLink {
    fn from(link: &Link) -> Self {
        Self {
            to: link.to_id.clone(),
            relation: link.relation.clone(),
        }
    }
}

/// A link read back from the index's link graph, with both ends.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphLink {
    pub from_id: String,
    pub to_id: String,
    pub relation: String,
}

/// A search result: note metadata (never the body) and a blended score.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created: String,
    pub score: f64,
}

/// A stable hash of a note body, so a later reindex can tell whether it changed (FNV-1a, hex).
#[must_use]
pub fn body_hash(body: &str) -> String {
    let hash = body.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_hash_is_stable_and_sensitive() {
        assert_eq!(body_hash("a"), body_hash("a"));
        assert_ne!(body_hash("a"), body_hash("b"));
        assert_eq!(body_hash(""), "cbf29ce484222325");
    }
}
