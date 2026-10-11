# Getting started

## Requirements

For local Rust development, install a stable Rust toolchain with Cargo, Python 3, and Git. The repository CI additionally checks Linux, macOS, and Windows. No provider credential is required for the deterministic local features.

## Verify the checkout

```sh
python3 eval/benchmark.py
python3 eval/release_gate.py
cargo test --workspace --all-targets
```

If Cargo is unavailable, the Python checks still validate the offline corpus and release inputs; do not interpret that as Rust test success.

## Runtime state

By default the CLI writes runtime state below `.conduit` in the current directory. Set `CONDUIT_HOME` to place blobs and session logs elsewhere. Set `CONDUIT_WORKSPACE` when the command is launched outside the repository being searched or edited.

```sh
export CONDUIT_HOME="$HOME/.local/state/conduit"
export CONDUIT_WORKSPACE="$PWD"
```

Raw output is content-addressed. Keep the state directory private because it may contain sensitive command output, even though future redaction will be applied at the boundary.

## First workflow

```sh
conduit run -- cargo test
conduit find "EventLog" 10
# Use the hash returned by find after confirming the file is unchanged.
conduit edit crates/scope/src/lib.rs HASH 'old text' 'new text'
```

Always review an edit diff and run the narrowest relevant test before a broader verification pass.
