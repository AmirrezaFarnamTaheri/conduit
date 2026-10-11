#!/usr/bin/env python3
"""Exercise the Rust reducer on a real corpus, not a Python imitation.

This is a word-count regression gate. It does not measure model token usage,
agent task success, or end-to-end latency. Those require paired trace studies.
"""
import argparse
import csv
import os
from pathlib import Path
import subprocess
import sys

CORPUS = Path(__file__).resolve().parent / "corpus"


def parse_measurement(stdout: str) -> tuple[int, int, bool]:
    fields = {}
    for item in stdout.strip().split():
        if "=" in item:
            key, value = item.split("=", 1)
            fields[key] = value
    if fields.get("mode") != "rust_cli_effective_word_estimate":
        raise ValueError("unexpected benchmark mode or incompatible Conduit binary")
    raw = int(fields["raw_words"])
    shown = int(fields["presented_words"])
    if raw <= 0 or shown < 0 or shown > raw:
        raise ValueError("invalid or empty raw/presented word counts")
    if fields.get("compact") not in ("true", "false"):
        raise ValueError("missing or invalid reduction flag")
    if fields.get("raw_recoverable") != "true":
        raise ValueError("raw bytes were not verified as recoverable")
    if fields["compact"] == "false" and shown != raw:
        raise ValueError("no compaction happened but savings were reported")
    return raw, shown, fields["compact"] == "true"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    filename = "conduit.exe" if os.name == "nt" else "conduit"
    parser.add_argument("--binary", type=Path, default=Path("target/debug") / filename)
    parser.add_argument("--budget", type=int, default=700)
    args = parser.parse_args()
    if args.budget <= 0 or args.budget > 100_000:
        parser.error("budget must be between 1 and 100000")
    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error(f"the compiled Conduit executable was not found: {binary}. Run cargo build --bin conduit")
    paths = sorted(CORPUS.glob("*.log"))
    if not paths:
        parser.error("benchmark corpus is empty; no evidence was measured")
    rows = []
    for path in paths:
        if not path.read_bytes().strip():
            parser.error(f"empty benchmark file: {path}")
        try:
            result = subprocess.run([str(binary), "bench", str(path), str(args.budget)],
                                    check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                    text=True, timeout=30)
            raw, shown, compact = parse_measurement(result.stdout)
        except (OSError, ValueError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
            parser.error(f"benchmark failed for {path.name}: {error}")
        rows.append((path.name, raw, shown, raw - shown, compact, round(1 - shown / raw, 4)))
    writer = csv.writer(sys.stdout)
    writer.writerow(("corpus", "raw_words", "effective_shown_words", "saved_word_estimate", "compact", "reduction"))
    writer.writerows(rows)
    total_raw = sum(row[1] for row in rows)
    total_shown = sum(row[2] for row in rows)
    writer.writerow(("TOTAL", total_raw, total_shown, total_raw - total_shown, "mixed", round(1 - total_shown / total_raw, 4)))
    print("evidence=rust_cli_presentation_path_including_recovery_hint_not_model_tokens_or_agent_outcomes", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
