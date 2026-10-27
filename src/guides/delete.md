# azk delete

  azk delete <id>

Removes the note's file, its index entry, and every link referencing it in
either direction.

Returns `{ id, deleted: true }`, or `{ error }` (exit 0) if the note doesn't
exist. A missing id argument prints usage and exits 1.
