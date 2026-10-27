//! The derived SQLite index at `.azk/azk.db`: full-text search, embeddings and the link graph.

use crate::model::{GraphLink, Hit, Link, Note, body_hash};
use rusqlite::{Connection, OptionalExtension, params};

pub const DIR: &str = ".azk";
pub const PATH: &str = ".azk/azk.db";

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS notes (
        id TEXT PRIMARY KEY,
        title TEXT NOT NULL,
        tags TEXT NOT NULL,
        created TEXT NOT NULL,
        body_hash TEXT NOT NULL,
        embedding BLOB
    );
    CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(id UNINDEXED, title, body, tags);
    CREATE TABLE IF NOT EXISTS links (
        from_id TEXT NOT NULL,
        to_id TEXT NOT NULL,
        relation TEXT NOT NULL,
        PRIMARY KEY (from_id, to_id, relation)
    );";

/// # Errors
///
/// Fails if the schema cannot be created.
pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA)
}

/// Opens the on-disk index, creating it if needed, and runs `f` against it.
///
/// # Errors
///
/// Fails with the database's message.
pub fn with<T>(f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T, String> {
    let open = || -> Result<Connection, String> {
        std::fs::create_dir_all(DIR).map_err(|e| e.to_string())?;
        Connection::open(PATH).map_err(|e| e.to_string())
    };
    let conn = open()?;
    init(&conn).map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

fn to_blob(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn from_blob(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum();
    let norm = |v: &[f32]| v.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt();
    let denom = norm(a) * norm(b);
    if denom == 0.0 { 0.0 } else { dot / denom }
}

/// # Errors
///
/// Fails on a database error.
pub fn upsert(conn: &Connection, note: &Note, embedding: Option<&[f32]>) -> rusqlite::Result<()> {
    let tags = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".into());
    conn.execute(
        "INSERT INTO notes (id, title, tags, created, body_hash, embedding)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
             title = excluded.title, tags = excluded.tags, created = excluded.created,
             body_hash = excluded.body_hash,
             embedding = COALESCE(excluded.embedding, notes.embedding)",
        params![
            note.id,
            note.title,
            tags,
            note.created,
            body_hash(&note.body),
            embedding.map(to_blob)
        ],
    )?;
    conn.execute("DELETE FROM notes_fts WHERE id = ?1", [&note.id])?;
    conn.execute(
        "INSERT INTO notes_fts (id, title, body, tags) VALUES (?1, ?2, ?3, ?4)",
        params![note.id, note.title, note.body, note.tags.join(" ")],
    )?;
    Ok(())
}

/// # Errors
///
/// Fails on a database error.
pub fn indexed_hash(conn: &Connection, id: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT body_hash FROM notes WHERE id = ?1", [id], |r| {
        r.get(0)
    })
    .optional()
}

/// # Errors
///
/// Fails on a database error.
pub fn add_links(conn: &Connection, from: &str, links: &[Link]) -> rusqlite::Result<()> {
    for link in links {
        conn.execute(
            "INSERT OR IGNORE INTO links (from_id, to_id, relation) VALUES (?1, ?2, ?3)",
            params![from, link.to_id, link.relation],
        )?;
    }
    Ok(())
}

/// # Errors
///
/// Fails on a database error.
pub fn replace_links(conn: &Connection, from: &str, links: &[Link]) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM links WHERE from_id = ?1", [from])?;
    add_links(conn, from, links)
}

/// # Errors
///
/// Fails on a database error.
pub fn links_for(conn: &Connection, id: &str) -> rusqlite::Result<Vec<GraphLink>> {
    let mut stmt = conn.prepare(
        "SELECT from_id, to_id, relation FROM links
         WHERE from_id = ?1 OR to_id = ?1 ORDER BY from_id, to_id, relation",
    )?;
    stmt.query_map([id], |r| {
        Ok(GraphLink {
            from_id: r.get(0)?,
            to_id: r.get(1)?,
            relation: r.get(2)?,
        })
    })?
    .collect()
}

/// # Errors
///
/// Fails on a database error.
pub fn delete(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM notes WHERE id = ?1", [id])?;
    conn.execute("DELETE FROM notes_fts WHERE id = ?1", [id])?;
    conn.execute("DELETE FROM links WHERE from_id = ?1 OR to_id = ?1", [id])?;
    Ok(())
}

fn hit(r: &rusqlite::Row<'_>, score: f64) -> rusqlite::Result<Hit> {
    let tags: String = r.get(2)?;
    Ok(Hit {
        id: r.get(0)?,
        title: r.get(1)?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        created: r.get(3)?,
        score,
    })
}

/// Turns free text into an FTS5 query matching any of its words.
fn fts_query(query: &str) -> Option<String> {
    let words: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{w}\""))
        .collect();
    (!words.is_empty()).then(|| words.join(" OR "))
}

/// # Errors
///
/// Fails on a database error.
pub fn keyword_search(conn: &Connection, query: &str, limit: usize) -> rusqlite::Result<Vec<Hit>> {
    let Some(query) = fts_query(query) else {
        return Ok(Vec::new());
    };
    let mut stmt = conn.prepare(
        "SELECT n.id, n.title, n.tags, n.created FROM notes_fts
         JOIN notes n ON n.id = notes_fts.id
         WHERE notes_fts MATCH ?1 ORDER BY bm25(notes_fts) LIMIT ?2",
    )?;
    let limit = i64::try_from(limit).unwrap_or(i64::MAX);
    stmt.query_map(params![query, limit], |r| hit(r, 0.0))?
        .collect()
}

/// # Errors
///
/// Fails on a database error.
pub fn semantic_search(
    conn: &Connection,
    vector: &[f32],
    limit: usize,
) -> rusqlite::Result<Vec<Hit>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, tags, created, embedding FROM notes WHERE embedding IS NOT NULL",
    )?;
    let mut hits = stmt
        .query_map([], |r| {
            let blob: Vec<u8> = r.get(4)?;
            let stored = from_blob(&blob);
            let score = if stored.len() == vector.len() {
                cosine(vector, &stored)
            } else {
                f64::NEG_INFINITY
            };
            Ok((hit(r, score)?, score))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    hits.retain(|(_, score)| score.is_finite());
    hits.sort_by(|a, b| b.1.total_cmp(&a.1));
    Ok(hits.into_iter().map(|(h, _)| h).take(limit).collect())
}

/// # Errors
///
/// Fails on a database error.
pub fn prune(conn: &Connection, keep: &[String]) -> rusqlite::Result<usize> {
    let mut stmt = conn.prepare("SELECT id FROM notes")?;
    let indexed = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let gone: Vec<_> = indexed.iter().filter(|id| !keep.contains(id)).collect();
    for id in &gone {
        delete(conn, id)?;
    }
    Ok(gone.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        conn
    }

    fn note(id: &str, title: &str, body: &str) -> Note {
        Note {
            id: id.into(),
            title: title.into(),
            tags: vec!["t".into()],
            created: "2026-01-01T00:00:00.000Z".into(),
            updated: "2026-01-01T00:00:00.000Z".into(),
            links: Vec::new(),
            body: body.into(),
        }
    }

    fn link(to: &str) -> Link {
        Link {
            to_id: to.into(),
            relation: "r".into(),
        }
    }

    #[test]
    fn keyword_search_finds_by_body_and_title_and_ignores_punctuation() {
        let conn = conn();
        upsert(&conn, &note("a", "Walks", "state machines"), None).unwrap();
        upsert(&conn, &note("b", "Other", "nothing"), None).unwrap();

        let hits = keyword_search(&conn, "machines? (walks)", 5).unwrap();

        assert_eq!(
            hits.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
            ["a"]
        );
        assert_eq!(hits[0].tags, ["t"]);
        assert!(keyword_search(&conn, "???", 5).unwrap().is_empty());
    }

    #[test]
    fn upsert_without_a_vector_keeps_the_stored_one() {
        let conn = conn();
        upsert(&conn, &note("a", "A", "x"), Some(&[1.0, 0.0])).unwrap();

        upsert(&conn, &note("a", "A2", "y"), None).unwrap();

        let hits = semantic_search(&conn, &[1.0, 0.0], 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "A2");
        assert_ne!(indexed_hash(&conn, "a").unwrap(), Some(body_hash("x")));
    }

    #[test]
    fn semantic_search_ranks_by_cosine_and_skips_other_dimensions() {
        let conn = conn();
        upsert(&conn, &note("near", "n", "x"), Some(&[1.0, 0.1])).unwrap();
        upsert(&conn, &note("far", "f", "x"), Some(&[0.0, 1.0])).unwrap();
        upsert(&conn, &note("odd", "o", "x"), Some(&[1.0, 0.0, 0.0])).unwrap();

        let hits = semantic_search(&conn, &[1.0, 0.0], 5).unwrap();

        assert_eq!(
            hits.iter().map(|h| h.id.as_str()).collect::<Vec<_>>(),
            ["near", "far"]
        );
    }

    #[test]
    fn links_are_idempotent_and_found_in_both_directions() {
        let conn = conn();
        add_links(&conn, "a", &[link("b")]).unwrap();
        add_links(&conn, "a", &[link("b")]).unwrap();

        assert_eq!(links_for(&conn, "a").unwrap().len(), 1);
        assert_eq!(links_for(&conn, "b").unwrap()[0].from_id, "a");

        replace_links(&conn, "a", &[link("c")]).unwrap();
        assert_eq!(links_for(&conn, "b").unwrap(), []);
    }

    #[test]
    fn delete_removes_entry_and_links_both_ways() {
        let conn = conn();
        upsert(&conn, &note("a", "A", "x"), None).unwrap();
        add_links(&conn, "b", &[link("a")]).unwrap();
        add_links(&conn, "a", &[link("c")]).unwrap();

        delete(&conn, "a").unwrap();

        assert_eq!(indexed_hash(&conn, "a").unwrap(), None);
        assert!(links_for(&conn, "a").unwrap().is_empty());
        assert!(keyword_search(&conn, "x", 5).unwrap().is_empty());
    }

    #[test]
    fn prune_drops_entries_not_kept() {
        let conn = conn();
        upsert(&conn, &note("a", "A", "x"), None).unwrap();
        upsert(&conn, &note("b", "B", "x"), None).unwrap();

        assert_eq!(prune(&conn, &["a".to_string()]).unwrap(), 1);
        assert_eq!(indexed_hash(&conn, "b").unwrap(), None);
        assert!(indexed_hash(&conn, "a").unwrap().is_some());
    }
}
