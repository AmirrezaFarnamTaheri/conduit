# Hash-anchored batch edits

Conduit accepts a JSON manifest containing a bounded list of source edits. The
manifest itself is data, not shell code. No program or tool is executed by this
operation. Relative paths resolve under `CONDUIT_WORKSPACE` (default `.`).

## Manifest

```json
{
  "operations": [
    {
      "type": "search_replace",
      "path": "src/main.rs",
      "expected_hash": "<entire 64-character BLAKE3 hash from conduit peek>",
      "old": "old code fragment",
      "new": "replacement code fragment"
    },
    {
      "type": "replace_range",
      "path": "src/lib.rs",
      "expected_hash": "<hash from conduit peek>",
      "start": 0,
      "end": 5,
      "new": "hello"
    }
  ]
}
```

`start` and `end` are **0-based UTF-8 byte offsets** in the original source,
with an exclusive end. `search_replace` requires exactly one occurrence of the
`old` string. There can be only one edit per canonical target in a batch;
combine adjacent changes into one updated range when necessary. Manifest
placeholders are not valid hashes and must be replaced before execution.

```sh
export CONDUIT_WORKSPACE="$PWD"
conduit peek src/main.rs 1 40
conduit batch-edit --check /tmp/reviewed-change.json
conduit batch-edit /tmp/reviewed-change.json
```

## Guarantees and limits

- `--check` and a real commit share target/path/hash/size validation.
- Preflight does **not** reserve a target against modifications by other tools.
- Batch size: 1–128 operations; each source/replacement <=8 MiB; total input
  plus replacement payload <=64 MiB; JSON manifest <=1 MiB.
- Rejects paths outside the workspace and edits targeting `.git` or `.conduit`.
- Rejects duplicate canonical targets and hardlinked files on Unix, because
  the rename implementation would otherwise silently separate aliases.
- Staging uses same-directory files and a process-lifetime cooperative OS lock;
  per-file source hashes are rechecked before replacement.
- Multi-file all-or-nothing power-loss/crash recovery is **not** implemented.
  Even successful dry-run checks do not change that limitation.
- Concurrent uncooperative external writers, symbolic-link races and filesystem
  semantics remain potential threats. This tool is not a filesystem sandbox.

Do not use the batch API for untrusted source writes or multi-file changes
requiring transactional crash recovery. For such operations use a versioned
source-control workflow and a supervisory transaction/recovery mechanism.
