# azk

A Zettelkasten for coding agents: atomic, explicitly-linked notes that let
design decisions and gotchas compound across sessions instead of being
rediscovered every time.

- `notes/<id>.md` — Markdown with YAML frontmatter. The version-controlled
  source of truth; safe to edit by hand or in any Markdown editor.
- `.azk/azk.db` — a derived, gitignored SQLite index (full-text search,
  optional semantic embeddings, link graph). Never read or write it directly;
  it is always rebuildable from `notes/` with `azk reindex`.

## Verbs

  azk search "<query>" [limit]       Rank notes against a query (metadata only)
  azk create                         Create a note from JSON on stdin
  azk get <id>                       Fetch one note, with body and links
  azk update <id>                    Partially update a note from JSON on stdin
  azk delete <id>                    Delete a note and every link touching it
  azk link <fromId> <toId> "<rel>"   Link two existing notes
  azk reindex                        Rebuild the index from notes/
  azk help [verb]                    Print this guide, or one verb's guide

Run `azk help <verb>` for that verb's exact output shape and gotchas.

## Output conventions

- Every verb except `help` prints a single JSON value to stdout. Usage
  messages go to stderr.
- **Exit code isn't a reliable success signal.** A bad invocation (missing
  argument, malformed JSON on stdin) exits 1. A failure *inside* a verb — a
  note not found, a write or embed failure — prints `{ "error": "..." }`
  and still exits 0. Always check the JSON for a top-level `error` key.

## Typical workflow

1. Once per project: make sure `.azk/` is gitignored, then `azk reindex` —
   the index is gitignored, so a fresh checkout starts without one. The azk
   README also shows optional git hooks that keep it in step.
2. Before non-trivial work: `azk search "<task summary>"`, then `azk get`
   only the 1-3 hits that look relevant. Treat them as prior art.
3. After the work: `azk create` one note per non-obvious decision or gotcha —
   not a restatement of the diff. Link it to related notes.
4. When a note turns out stale or wrong: `azk update` it (or
   `azk delete` it) rather than leaving a contradicting note beside it.
5. After hand-editing anything under `notes/`: `azk reindex`.

## Semantic search

Search blends keyword (FTS5) ranking with semantic similarity when an
OpenAI-compatible embeddings host is reachable — by default
`http://localhost:11434/v1` with model `embeddinggemma:latest`. Override
either with the `EMBEDDING_HOST` / `EMBEDDING_MODEL` environment variables
(a `.env` file in the project works). With no host, search degrades to
keyword-only rather than failing.
