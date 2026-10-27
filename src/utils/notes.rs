//! Markdown notes with YAML frontmatter under `notes/<id>.md`.

use crate::model::{Note, NoteLink};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

pub const DIR: &str = "notes";

#[derive(Serialize, Deserialize)]
struct Frontmatter {
    id: String,
    title: String,
    #[serde(default)]
    tags: Vec<String>,
    created: String,
    updated: String,
    #[serde(default)]
    links: Vec<NoteLink>,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// Quotes a timestamp so it stays a string for every YAML reader, unlike an unquoted ISO date.
fn quote_timestamps(yaml: String) -> String {
    yaml.lines()
        .map(|line| match line.split_once(": ") {
            Some((key @ ("created" | "updated"), value)) if !value.starts_with(['\'', '"']) => {
                format!("{key}: '{value}'")
            }
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn render(note: &Note) -> String {
    let front = Frontmatter {
        id: note.id.clone(),
        title: note.title.clone(),
        tags: note.tags.clone(),
        created: note.created.clone(),
        updated: note.updated.clone(),
        links: note.links.clone(),
    };
    let yaml = serde_yaml::to_string(&front).unwrap_or_default();
    format!("---\n{}\n---\n{}\n", quote_timestamps(yaml), note.body)
}

/// # Errors
///
/// Fails if the text has no frontmatter or the frontmatter is not valid.
pub fn parse(text: &str) -> io::Result<Note> {
    let rest = text
        .strip_prefix("---\n")
        .ok_or_else(|| invalid("missing frontmatter"))?;
    let (yaml, body) = rest
        .split_once("\n---\n")
        .or_else(|| rest.strip_suffix("\n---").map(|yaml| (yaml, "")))
        .ok_or_else(|| invalid("unterminated frontmatter"))?;
    let front: Frontmatter = serde_yaml::from_str(yaml).map_err(|e| invalid(e.to_string()))?;
    Ok(Note {
        id: front.id,
        title: front.title,
        tags: front.tags,
        created: front.created,
        updated: front.updated,
        links: front.links,
        body: body.strip_suffix('\n').unwrap_or(body).to_string(),
    })
}

fn path(id: &str) -> PathBuf {
    Path::new(DIR).join(format!("{id}.md"))
}

/// # Errors
///
/// Fails on anything but a missing file, which is `Ok(None)`.
pub fn read(id: &str) -> io::Result<Option<Note>> {
    match std::fs::read_to_string(path(id)) {
        Ok(text) => parse(&text).map(Some),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// # Errors
///
/// Fails if the directory or file cannot be written.
pub fn write(note: &Note) -> io::Result<()> {
    std::fs::create_dir_all(DIR)?;
    std::fs::write(path(&note.id), render(note))
}

/// # Errors
///
/// Fails if the directory exists but cannot be read; a missing one is no notes.
pub fn list() -> io::Result<Vec<String>> {
    let entries = match std::fs::read_dir(DIR) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut ids = Vec::new();
    for entry in entries {
        let name = entry?.file_name();
        if let Some(id) = name.to_string_lossy().strip_suffix(".md") {
            ids.push(id.to_string());
        }
    }
    ids.sort();
    Ok(ids)
}

/// # Errors
///
/// Fails if the file cannot be removed.
pub fn remove(id: &str) -> io::Result<()> {
    std::fs::remove_file(path(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note() -> Note {
        Note {
            id: "202610272204371565e63".into(),
            title: "A: title with 'quotes'".into(),
            tags: vec!["a".into(), "b".into()],
            created: "2026-10-27T22:04:37.156Z".into(),
            updated: "2026-10-27T22:04:37.156Z".into(),
            links: vec![NoteLink {
                to: "2026102723174092576db".into(),
                relation: "builds-on".into(),
            }],
            body: "First line.\n\nSecond: --- not a fence.".into(),
        }
    }

    #[test]
    fn round_trips() {
        assert_eq!(parse(&render(&note())).unwrap(), note());
    }

    #[test]
    fn timestamps_are_written_quoted() {
        let text = render(&note());

        assert!(
            text.contains("created: '2026-10-27T22:04:37.156Z'"),
            "{text}"
        );
        assert!(
            text.contains("updated: '2026-10-27T22:04:37.156Z'"),
            "{text}"
        );
    }

    #[test]
    fn reads_unquoted_timestamps_and_missing_optionals() {
        let text = "---\nid: x\ntitle: T\ncreated: 2026-09-01T00:00:00.000Z\nupdated: 2026-09-01T00:00:00.000Z\n---\nbody\n";

        let note = parse(text).unwrap();

        assert_eq!(note.created, "2026-09-01T00:00:00.000Z");
        assert!(note.tags.is_empty() && note.links.is_empty());
        assert_eq!(note.body, "body");
    }

    #[test]
    fn rejects_text_without_frontmatter() {
        assert!(parse("just text").is_err());
    }
}
