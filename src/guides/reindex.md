# azk reindex

  azk reindex

Rebuilds `.azk/azk.db` from `notes/*.md`. Returns
`{ indexed, updated, removed, total }`.

- Re-embeds only notes whose body changed, but re-upserts every note and
  rebuilds its links, so a hand-edit touching only frontmatter links is still
  picked up.
- Drops index entries for notes whose file no longer exists.
- Always safe: `.azk/` can be deleted and rebuilt this way at any time.

Gotchas:
- A note whose frontmatter has an *unquoted* ISO timestamp
  (`created: 2026-09-01T00:00:00.000Z`) parses as a YAML date, not a string,
  and the index rejects it —
  `{ "error": "Provided value cannot be bound to SQLite parameter 4." }` —
  abandoning the rest of the queue. azk always quotes timestamps it writes; a
  hand-written or editor-generated note may not.
