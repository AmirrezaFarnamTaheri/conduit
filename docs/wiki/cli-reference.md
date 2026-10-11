# CLI reference

## `run -- <command> [args...]`

Runs a child process, captures output, stores raw bytes when possible, and presents reduced output. The child exit code is returned unchanged. Interactive programs and exact stdout/stderr temporal interleaving are not yet fully supported by the current `Command::output` boundary.

## `find <query> [limit]`

Searches a deterministic workspace snapshot. Each hit contains a relative path, one-based line number, source line, and content hash. Empty queries return no results. `CONDUIT_WORKSPACE` selects the root.

## `edit <path> <expected-hash> <old> <new>`

Performs a unique search/replace only if the complete file hash matches. Parent traversal, absolute paths, symlink escapes, ambiguous matches, and duplicate transaction paths are rejected. Edits use a workspace lock and staged replacement.

## `store <text>` and `expand <handle>`

Stores text and prints an output handle, or retrieves a previously stored blob. Expired or unknown handles fail clearly; output handles are not guessed.

## `record`, `replay`, `stats`

Record a session accounting event, print its append-only events, or compute typed token totals:

```sh
conduit record demo lens 1000 400
conduit stats demo
conduit replay demo
```

## `bench`

Runs the small local corpus smoke check. The authoritative offline benchmark is `python3 eval/benchmark.py`.

## Environment

- `CONDUIT_HOME`: blob and session root.
- `CONDUIT_WORKSPACE`: atlas/edit root.

Never pass secrets as command-line arguments; process listings may expose them.
