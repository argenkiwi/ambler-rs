# Program Specifications

This program is the `get` verb of the `azk` command line: it fetches one Zettelkasten note and its links. Notes persist as Markdown files with YAML frontmatter under `notes/<id>.md` (the source of truth); `.azk/azk.db` is a derived, gitignored SQLite index (full-text search, optional embeddings and a link graph), fully rebuildable from the notes by the `reindex` walk.

The note itself is read from its Markdown file; only the links come from the index.

Every node prints at most one JSON value to stdout. A missing note is a failure inside the verb: it prints `{ "error": ... }` and still exits 0. Only a bad invocation — no id — reaches `USAGE` and exits 1.

## Nodes

### Id Arg
- Initial node of the program.
- Shared with the `update` and `delete` walks.
- Reads a note id from the first argument, building its usage line from the verb the walk seeded, so one node serves all three.
- If an id was given, transitions to `EXISTS_CHECK`. Otherwise transitions to `USAGE`.

### Exists Check
- Shared with the `update` and `delete` walks.
- Reads the note by id and attaches it, so downstream nodes don't read it again.
- If found, transitions to `GET`. Otherwise prints `{ "error": ... }` and terminates.

### Get
- Terminal node of the program.
- Prints the note the existence check already read, together with every link touching it in either direction.
- Terminates.

### Usage
- Terminal node, reached only from `ID_ARG`.
- Shared with every other walk that can be invoked badly.
- Prints the pending usage message to stderr, leaving stdout for the single JSON value a successful run produces.
- Terminates. The process then exits 1.

## Shared State

`verb` and `args` are seeded from the command line. `verb` exists because `ID_ARG` builds its usage line from it, which is what lets one node serve `get`, `update` and `delete`.

- `verb`: Always `get` here.
- `args`: The verb's arguments.
- `usage`: The message `USAGE` should print. Its presence is what makes the run exit 1.
- `error`: The message a failing node already printed.
- `id`: The note being fetched.
- `note`: That note, once read from disk.
