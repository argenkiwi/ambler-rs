# Program Specifications

This program is the `delete` verb of the `azk` command line: it removes a Zettelkasten note. Notes persist as Markdown files with YAML frontmatter under `notes/<id>.md` (the source of truth); `.azk/azk.db` is a derived, gitignored SQLite index (full-text search, optional embeddings and a link graph), fully rebuildable from the notes by the `reindex` walk.

Deleting removes the Markdown file, the index entry, and every link referencing the note in either direction — including links pointing *at* it from notes that still exist, which is why a later `reindex` doesn't resurrect them.

Every node prints at most one JSON value to stdout. A missing note is a failure inside the verb: it prints `{ "error": ... }` and still exits 0. Only a bad invocation — no id — reaches `USAGE` and exits 1.

## Nodes

### Id Arg
- Initial node of the program.
- Shared with the `get` and `update` walks.
- Reads a note id from the first argument, building its usage line from the verb the walk seeded, so one node serves all three.
- If an id was given, transitions to `EXISTS_CHECK`. Otherwise transitions to `USAGE`.

### Exists Check
- Shared with the `get` and `update` walks.
- Reads the note by id, so that deleting something that was never there reports an error rather than silently succeeding.
- If found, transitions to `DELETE`. Otherwise prints `{ "error": ... }` and terminates.

### Delete
- Terminal node of the program.
- Removes the note's Markdown file, its index entry, and any links referencing it in either direction. The existence check upstream has already ruled out a missing id.
- Prints the deletion confirmation and terminates.

### Usage
- Terminal node, reached only from `ID_ARG`.
- Shared with every other walk that can be invoked badly.
- Prints the pending usage message to stderr, leaving stdout for the single JSON value a successful run produces.
- Terminates. The process then exits 1.

## Shared State

`verb` and `args` are seeded from the command line. `verb` exists because `ID_ARG` builds its usage line from it, which is what lets one node serve `get`, `update` and `delete`.

- `verb`: Always `delete` here.
- `args`: The verb's arguments.
- `usage`: The message `USAGE` should print. Its presence is what makes the run exit 1.
- `error`: The message a failing node already printed.
- `id`: The note being deleted.
- `note`: That note, read by the existence check. Nothing downstream reads it — `DELETE` works from the id alone — but the check attaches it all the same, since it is the node the other two walks share.
