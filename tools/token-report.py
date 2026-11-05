# ADR: Reads omp session transcripts (JSONL, one `message.usage` per assistant request) and reports spend per agent; stdlib only.
# ADR: Cost figures are the harness's own `usage.cost` values; "rewrites" counts requests whose cache write exceeded half their context (an expired or broken cache).
"""Per-agent token and cost report for omp sessions of this repository.

Usage:
  python tools/token-report.py              # newest session of this repository
  python tools/token-report.py --all        # every session of this repository
  python tools/token-report.py <session.jsonl | session-dir>
"""

import argparse
import json
import sys
from pathlib import Path

KINDS = ("input", "output", "cacheRead", "cacheWrite")


def sessions_root(repo: Path) -> Path:
    # omp names a project's folder after its path relative to home, with separators as dashes.
    rel = repo.resolve().relative_to(Path.home())
    return Path.home() / ".omp" / "agent" / "sessions" / ("-" + "-".join(rel.parts))


def transcripts(session_file: Path) -> list[Path]:
    """The lead transcript plus its subagent transcripts in the sibling folder of the same name."""
    files = [session_file]
    sub = session_file.with_suffix("")
    if sub.is_dir():
        files += sorted(sub.glob("*.jsonl"))
    return files


def usages(path: Path) -> list[dict]:
    out = []
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            message = record.get("message")
            if isinstance(message, dict) and isinstance(message.get("usage"), dict):
                out.append(message["usage"])
    return out


def report(files: list[Path]) -> None:
    totals = dict.fromkeys(KINDS, 0.0)
    print(f"{'agent':24} {'req':>4} {'$total':>7} {'$read':>6} {'$write':>6} {'$out':>6} "
          f"{'maxK':>5} {'avgK':>5} {'rewrites':>8}  ttl")
    for path in files:
        rows = usages(path)
        if not rows:
            continue
        cost = {k: sum(r.get("cost", {}).get(k, 0.0) for r in rows) for k in KINDS}
        for k in KINDS:
            totals[k] += cost[k]
        context = [r.get("input", 0) + r.get("cacheRead", 0) + r.get("cacheWrite", 0) for r in rows]
        rewrites = sum(1 for r, c in zip(rows[1:], context[1:]) if c > 30_000 and r.get("cacheWrite", 0) > c / 2)
        ttl: dict[str, int] = {}
        for r in rows:
            for key, value in (r.get("cttl") or {}).items():
                ttl[key] = ttl.get(key, 0) + value
        name = "lead" if path == files[0] else path.stem
        print(f"{name[:24]:24} {len(rows):4} {sum(cost.values()):7.2f} {cost['cacheRead']:6.2f} "
              f"{cost['cacheWrite']:6.2f} {cost['output']:6.2f} {max(context) // 1000:5} "
              f"{sum(context) // len(context) // 1000:5} {rewrites:8}  {ttl}")
    total = sum(totals.values())
    if total:
        shares = ", ".join(f"{k} {v:.2f} ({v / total:.0%})" for k, v in totals.items())
        print(f"total ${total:.2f}: {shares}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("session", nargs="?", help="a session .jsonl file or its subagent folder")
    parser.add_argument("--all", action="store_true", help="report every session of this repository")
    args = parser.parse_args()

    if args.session:
        target = Path(args.session)
        session_file = target if target.suffix == ".jsonl" else target.with_suffix(".jsonl")
        if not session_file.is_file():
            print(f"no session transcript at {session_file}", file=sys.stderr)
            return 2
        sessions = [session_file]
    else:
        root = sessions_root(Path(__file__).resolve().parent.parent)
        sessions = sorted(root.glob("*.jsonl"), key=lambda p: p.stat().st_mtime)
        if not sessions:
            print(f"no sessions under {root}", file=sys.stderr)
            return 2
        if not args.all:
            sessions = sessions[-1:]

    for session_file in sessions:
        print(f"== {session_file.name}")
        report(transcripts(session_file))
    return 0


if __name__ == "__main__":
    sys.exit(main())
