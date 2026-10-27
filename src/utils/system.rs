use super::{Clock, Embedder, Index, NoteStore, Out, Stdin, clock, embed, index, notes};
use crate::model::{GraphLink, Hit, Link, Note};
use std::io::{self, Read};

/// The real world: the terminal, the working directory's notes and index, the embeddings host.
#[derive(Clone, Copy, Debug, Default)]
pub struct System;

impl Out for System {
    fn print(&self, message: &str) {
        println!("{message}");
    }
    fn eprint(&self, message: &str) {
        eprintln!("{message}");
    }
}

impl Stdin for System {
    fn read_stdin(&self) -> io::Result<String> {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        Ok(input)
    }
}

impl Clock for System {
    fn timestamp(&self) -> String {
        clock::timestamp()
    }
    fn new_id(&self) -> String {
        clock::new_id()
    }
}

impl NoteStore for System {
    fn read(&self, id: &str) -> io::Result<Option<Note>> {
        notes::read(id)
    }
    fn write(&self, note: &Note) -> io::Result<()> {
        notes::write(note)
    }
    fn list(&self) -> io::Result<Vec<String>> {
        notes::list()
    }
    fn remove(&self, id: &str) -> io::Result<()> {
        notes::remove(id)
    }
}

impl Embedder for System {
    fn embed(&self, text: &str) -> Option<Vec<f32>> {
        embed::embed(text)
    }
}

impl Index for System {
    fn upsert(&self, note: &Note, embedding: Option<&[f32]>) -> Result<(), String> {
        index::with(|c| index::upsert(c, note, embedding))
    }
    fn indexed_hash(&self, id: &str) -> Result<Option<String>, String> {
        index::with(|c| index::indexed_hash(c, id))
    }
    fn add_links(&self, from: &str, links: &[Link]) -> Result<(), String> {
        index::with(|c| index::add_links(c, from, links))
    }
    fn replace_links(&self, from: &str, links: &[Link]) -> Result<(), String> {
        index::with(|c| index::replace_links(c, from, links))
    }
    fn links_for(&self, id: &str) -> Result<Vec<GraphLink>, String> {
        index::with(|c| index::links_for(c, id))
    }
    fn delete(&self, id: &str) -> Result<(), String> {
        index::with(|c| index::delete(c, id))
    }
    fn keyword_search(&self, query: &str, limit: usize) -> Result<Vec<Hit>, String> {
        index::with(|c| index::keyword_search(c, query, limit))
    }
    fn semantic_search(&self, vector: &[f32], limit: usize) -> Result<Vec<Hit>, String> {
        index::with(|c| index::semantic_search(c, vector, limit))
    }
    fn prune(&self, keep: &[String]) -> Result<usize, String> {
        index::with(|c| index::prune(c, keep))
    }
}
