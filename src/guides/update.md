# azk update

  echo '{"body":"..."}' | azk update <id>

Partial update — any subset of `title`, `body`, `tags`. An empty object
`{}` is a valid no-op.

Returns `{ id, updated: true }` — not the updated fields themselves.

Gotchas:
- Only re-embeds when `body` changes.
- Malformed stdin JSON prints `{ "error": "invalid JSON on stdin" }` and exits
  1, as in `create`.
- An id that doesn't exist prints `{ error }` and exits 0; a missing id
  argument prints usage and exits 1.
- `links` can't be changed here — use `azk link`.
