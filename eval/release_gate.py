#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
required = [root / "Cargo.toml", root / "proto/conduit.proto", root / "deny.toml", root / "specs/threat-model.md"]
missing = [str(path.relative_to(root)) for path in required if not path.exists()]
if missing:
    raise SystemExit("missing release inputs: " + ", ".join(missing))
text = (root / "eval/corpus/sample.log").read_text()
if not re.search(r"error|passed|exit=", text, re.I):
    raise SystemExit("corpus lacks required signal coverage")
print("release metadata present; Rust gates must run in CI")
