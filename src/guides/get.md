# azk get

  azk get <id>

Returns the full note (including body) plus every link touching it in either
direction, or `{ error }` (exit 0) if the note doesn't exist.

A missing id prints usage to stderr and exits 1.
