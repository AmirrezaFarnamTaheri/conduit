#!/usr/bin/env python3
"""Deterministic Phase 0 corpus benchmark; external agents are intentionally optional."""
from pathlib import Path
import re

ROOT = Path(__file__).parent
CORPUS = ROOT / "corpus"
ERROR = re.compile(r"error|failed|panic|fatal|traceback|exception|FAILED", re.I)

def tokens(text: str) -> int:
    return len(re.findall(r"\S+", text))

def reduce_output(text: str) -> str:
    lines = text.splitlines()
    retained = [line for line in lines if ERROR.search(line) or re.search(r"\b\d+ (passed|failed)\b|exit=", line, re.I)]
    return "\n".join(dict.fromkeys(retained)) + ("\n" if retained else "")

def main() -> None:
    rows = []
    for path in sorted(CORPUS.glob("*.log")):
        raw = path.read_text()
        shown = reduce_output(raw)
        raw_tokens, shown_tokens = tokens(raw), tokens(shown)
        rows.append((path.name, raw_tokens, shown_tokens, round(1 - shown_tokens / raw_tokens, 4) if raw_tokens else 0))
    print("corpus,raw_tokens,presented_tokens,reduction")
    for row in rows:
        print(",".join(map(str, row)))

if __name__ == "__main__":
    main()
