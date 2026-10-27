# azk link

  azk link <fromId> <toId> "<relation>"

Links two existing notes with a short relation phrase (e.g. "builds on").

Returns `{ fromId, toId, relation, linked: true }`, or `{ error }` (exit 0)
if either id doesn't exist. Missing arguments print usage and exit 1.

If the relation you'd write reads like "supersedes" or "fixes X", consider
`azk update` on X instead of a new linked note.
