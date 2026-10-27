# Program Specifications

This program is the `reindex` verb of the `azk` command line: it rebuilds the derived index from the Markdown notes. Notes persist as Markdown files with YAML frontmatter under `notes/<id>.md` (the source of truth); `.azk/azk.db` is a derived, gitignored SQLite index (full-text search, optional embeddings and a link graph), and this walk is what makes "fully rebuildable from the notes" true. Deleting `.azk/` and running `reindex` is always safe.

It is the one verb that loops, and the one that takes no arguments — so it cannot be invoked badly, and has no `USAGE` node and no non-zero exit.

Every node prints at most one JSON value to stdout. A failure while indexing prints `{ "error": ... }`, abandons the rest of the queue, and still exits 0.

## Nodes

### List
- Initial node of the program.
- Seeds the loop from the Markdown files on disk — the source of truth — and zeroes the counters the later nodes accumulate into. Must stay the entry node: those counters are accumulated onto downstream, so entering at `NEXT` would produce nonsense totals rather than an error.
- Transitions to `NEXT`.

### Next
- Takes the next note id off the queue. The only node that decides how long the reindex runs.
- If one remains, transitions to `DETECT_CHANGE`. When the queue is empty, transitions to `PRUNE_ORPHANS`.

### Detect Change
- Reads the current note and decides whether it needs re-embedding, by comparing its body hash against the one the index holds, and counts it as newly indexed or as updated.
- The note is always re-upserted and its links always rebuilt downstream — only the embedding is conditional, because that is the one step costing a network round trip. The pending embed text is cleared when nothing changed, so the loop cannot carry one note's body into the next one's embedding.
- Transitions to `EMBED`. A file that vanished between listing and reading transitions back to `NEXT`.

### Embed
- Shared with the `search`, `create` and `update` walks.
- Embeds the pending text, if any. Never fails: an unreachable embeddings host yields no vector rather than an error, so a reindex without one still rebuilds the search index and the link graph.
- Writes the vector on every pass, never leaving the previous note's in place, which is the other half of what keeps the loop from mis-attributing an embedding.
- Transitions to `INDEX_UPSERT`.

### Index Upsert
- Shared with the `create` and `update` walks.
- Upserts the note into the index, hashing the body so the next reindex can tell whether it changed. No embedding leaves any previously stored vector alone, which is what makes an unchanged note cheap.
- If indexed, transitions to `REPLACE_LINKS`. Otherwise prints `{ "error": ... }` and terminates, abandoning the rest of the queue.

### Replace Links
- Rebuilds the current note's outgoing links in the index from its own frontmatter. Runs for every note, changed or not, so a hand-edit that only touched frontmatter links is still picked up.
- Transitions back to `NEXT`, closing the loop.

### Prune Orphans
- Drops index entries whose Markdown file no longer exists, keeping the index in step with the notes directory. This is what makes a note deleted by hand disappear from search.
- Transitions to `FINISH`.

### Finish
- Terminal node of the program.
- Prints the reindex summary, so the counters it reports are final.
- Terminates.

## Shared State

Nothing is seeded — `reindex` takes no arguments. It is also the only walk with no `usage` field, since there is no way to invoke it badly.

- `error`: The message a failing node already printed.
- `all_ids`: Every note on disk, as listed at the start. `PRUNE_ORPHANS` treats this as the set of ids that should survive.
- `remaining_ids`: Those still to process — the queue.
- `id`: The loop's cursor.
- `note`: The note that cursor points at.
- `text_to_embed`: Its body, queued only when the body changed, and cleared on every pass that doesn't.
- `embedding`: The resulting vector, or nothing, rewritten on every pass.
- `indexed`, `updated`, `total`: Counters seeded by `LIST`. `indexed` counts notes new to the index, `updated` ones whose body changed; a run where neither moves means the index was already in step.
- `removed`: How many orphans were pruned. Unlike the others it is not seeded, because only `PRUNE_ORPHANS` ever sets it and it always runs before `FINISH`.
