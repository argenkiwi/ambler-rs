# azk create

  echo '{"title":"...","body":"...","tags":["..."],"links":[{"toId":"...","relation":"..."}]}' | azk create

`title` and `body` are required; `tags` and `links` are optional.

Returns `{ id, title, tags, created, links }`, or `{ error }` (exit 0) if the
write or embedding fails.

Gotchas:
- Malformed or empty stdin JSON prints `{ "error": "invalid JSON on stdin" }`
  and exits 1; a missing `title`/`body` prints
  `{ "error": "title and body are required" }` and exits 1. Stdin problems
  never produce a `Usage:` line.
- `links` are not validated: a `toId` that doesn't exist is stored as a
  dangling link (unlike `azk link`, which checks both ids first). `reindex`
  won't clean it up either — it trusts each note's frontmatter links.
