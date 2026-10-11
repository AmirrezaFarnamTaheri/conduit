#!/usr/bin/env python3
"""Static release inputs sanity check. This does not replace build/security tests."""
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parents[1]
required = [root / "Cargo.toml", root / "proto/conduit.proto", root / "deny.toml", root / "specs/threat-model.md",
            root / "src/main.rs", root / "src/run.rs", root / "tests/cli.rs", root / "docs/wiki/batch-edit.md", root / "eval/benchmark.py", root / ".github/workflows/checks.yml"]
crate_names = ("store", "scope", "lens", "atlas", "edit", "ledger", "relay", "guard", "verify", "vault", "memory", "fanout")
for crate in crate_names:
    required.extend((root / f"crates/{crate}/Cargo.toml", root / f"crates/{crate}/src/lib.rs"))
missing = [str(path.relative_to(root)) for path in required if not path.is_file()]
if missing:
    raise SystemExit("missing required project inputs: " + ", ".join(missing))
files = sorted((root / "eval/corpus").glob("*.log"))
if not files:
    raise SystemExit("missing benchmark corpus: no .log fixtures")
for path in files:
    contents = path.read_text(encoding="utf-8")
    if not contents.strip() or not re.search(r"error|failed|panic|passed|exit=", contents, re.I):
        raise SystemExit(f"invalid or signal-free benchmark corpus: {path.name}")
print(f"static_release_inputs=present fixtures={len(files)} crates={len(crate_names)}")
print("not_verified=rust_compilation,subprocess_isolation,live_agent_performance", file=sys.stderr)
