# azk search

  azk search "<query>" [limit]

`limit` defaults to 5.

Returns `{ id, title, tags, created, score }[]`, best match first, or `[]`.
No note bodies — `azk get` the hits you actually need.

- Blends keyword (FTS5) and semantic ranking; degrades to keyword-only when no
  embeddings host is reachable.
- An empty result is not an error: it prints `[]` and exits 0.
- A missing query prints usage to stderr and exits 1.
