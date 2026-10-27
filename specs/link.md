# Program Specifications

This program is the `link` verb of the `azk` command line: it connects two existing Zettelkasten notes with a short relation phrase. Notes persist as Markdown files with YAML frontmatter under `notes/<id>.md` (the source of truth); `.azk/azk.db` is a derived, gitignored SQLite index (full-text search, optional embeddings and a link graph), fully rebuildable from the notes by the `reindex` walk.

A link is stored twice over: in the source note's own frontmatter, which is the source of truth, and in the index's link graph, which is what `get` and `reindex` read. `link` writes both and deliberately does **not** re-index the note otherwise — only the file and the link graph change, never the note's embedding or search entry.

Every node prints at most one JSON value to stdout. A missing note at either end is a failure inside the verb: it prints `{ "error": ... }` and still exits 0. Only a bad invocation — fewer than three arguments — reaches `USAGE` and exits 1.

## Nodes

### Input
- Initial node of the program.
- Reads the source id, target id and relation phrase from the arguments.
- If all three were given, transitions to `VALIDATE`. Otherwise transitions to `USAGE`.

### Validate
- Requires both ends of the link to already exist — unlike `create`'s links, which are not validated — then appends the link to the source note's frontmatter and restates it in the form the link graph wants.
- If both exist, transitions to `WRITE_NOTE`. Otherwise prints `{ "error": ... }` and terminates.

### Write Note
- Shared with the `create` and `update` walks.
- Persists the source note to its Markdown file — the source of truth, written before the derived link graph is touched.
- If written, transitions to `CREATE_LINKS`. Otherwise prints `{ "error": ... }` and terminates.

### Create Links
- Shared with the `create` walk.
- Records the pending link in the index's link graph, pointing out of the source note.
- If recorded, transitions to `FINISH`. Otherwise prints `{ "error": ... }` and terminates.

### Finish
- Terminal node of the program.
- Prints the link confirmation, which runs only once the source note and the link graph have both been written.
- Terminates.

### Usage
- Terminal node, reached only from `INPUT`.
- Shared with every other walk that can be invoked badly.
- Prints the pending usage message to stderr, leaving stdout for the single JSON value a successful run produces.
- Terminates. The process then exits 1.

## Shared State

`args` is the only seeded field — the walk takes it from the command line. Every other field is written by a node upstream of whoever reads it. There is no `text_to_embed`, `embedding` or `id` here, because `link` skips indexing entirely.

- `args`: The verb's arguments.
- `usage`: The message `USAGE` should print. Its presence is what makes the run exit 1.
- `error`: The message a failing node already printed.
- `from_id`, `to_id`, `relation`: The link input. `from_id` doubles as the note the pending link points out of, which is what lets `CREATE_LINKS` be shared with `create`.
- `note`: The source note, with the new link appended to its frontmatter.
- `links`: The same link in caller form, `{ to_id, relation }`, for the link graph — never the note's own `{ to, relation }` frontmatter form, which stays inside `note`. Unlike `create`, this is derived rather than caller-supplied, and it is not printed: `FINISH` reports the three input fields instead.
