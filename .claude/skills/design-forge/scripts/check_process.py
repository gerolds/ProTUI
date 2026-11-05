#!/usr/bin/env python3
"""Structural checker for a design-forge run.

Verifies that the process artifacts exist, were committed in order, and
satisfy the mechanical anti-theater invariants. It cannot judge quality;
it raises the cost of faking the process, it does not make faking impossible.

Usage: python3 check_process.py <forge_dir>
Exit code 0 = all checks pass, 1 = at least one FAIL.
"""

import re
import sys
from pathlib import Path

FAIL = 0


def report(ok: bool, name: str, detail: str = "") -> bool:
    global FAIL
    tag = "PASS" if ok else "FAIL"
    if not ok:
        FAIL = 1
    print(f"[{tag}] {name}" + (f" — {detail}" if detail else ""))
    return ok


def main(forge: Path) -> int:
    files = {p.name: p for p in forge.iterdir() if p.is_file()}

    # --- presence -----------------------------------------------------------
    required = ["00_sealed_lean.md", "01_frame.md", "02_rubric.md",
                "03_axes.md", "05_decision.md", "06_audit.md", "ledger.log"]
    for r in required:
        report(r in files, f"exists: {r}")

    options = sorted(n for n in files if re.match(r"03_option_[A-Z]", n))
    critiques = sorted(n for n in files if re.match(r"04_critique_[A-Z]", n))
    report(len(options) >= 3, "at least 3 options", f"found {len(options)}")
    opt_ids = {re.search(r"03_option_([A-Z])", n).group(1) for n in options}
    crit_ids = {re.search(r"04_critique_([A-Z])", n).group(1) for n in critiques}
    report(opt_ids == crit_ids and len(crit_ids) > 0,
           "every option has a critique", f"options={sorted(opt_ids)} critiques={sorted(crit_ids)}")

    # --- ledger ordering ----------------------------------------------------
    ledger = files.get("ledger.log")
    order = []
    if ledger:
        stamped = []
        for line in ledger.read_text(encoding="utf-8").splitlines():
            m = re.search(r"^(\S+)\s+CREATE\s+(\S+)", line)
            if m:
                stamped.append((m.group(1), m.group(2)))
                order.append(m.group(2))
        phase = lambda n: int(n[:2]) if n[:2].isdigit() else 99
        # verify by timestamp (ISO strings sort lexicographically), not line order
        by_time = [name for _, name in sorted(stamped)]
        tphases = [phase(n) for n in by_time if phase(n) != 99]
        report(tphases == sorted(tphases),
               "ledger CREATE timestamps are phase-ordered",
               "an artifact was created out of phase order (e.g. rubric after options)"
               if tphases != sorted(tphases) else "")
        missing = [r for r in required if r != "ledger.log" and r not in order]
        missing += [o for o in options + critiques if o not in order]
        report(not missing, "all artifacts logged in ledger",
               f"missing: {missing}" if missing else "")
        # silent amendments: files edited but never AMEND-logged can't be
        # detected from content alone; check that any AMEND lines carry a reason
        bad_amends = [l for l in ledger.read_text(encoding="utf-8").splitlines()
                      if "AMEND" in l and len(l.split("AMEND", 1)[1].split()) < 2]
        report(not bad_amends, "AMEND entries carry a reason",
               f"{len(bad_amends)} reasonless AMEND line(s)")

    # --- ID discipline ------------------------------------------------------
    frame = files.get("01_frame.md")
    rubric = files.get("02_rubric.md")
    valid_ids = set()
    if frame:
        valid_ids |= set(re.findall(r"\b([CAUF]\d+)\b", frame.read_text(encoding="utf-8")))
        report(bool(valid_ids), "frame defines C/A/U/F IDs",
               f"{len(valid_ids)} IDs found")
        report(any(i.startswith("F") for i in valid_ids),
               "frame includes failure conditions (F#)")
    if rubric:
        r_ids = set(re.findall(r"\b(R\d+)\b", rubric.read_text(encoding="utf-8")))
        valid_ids |= r_ids
        report(bool(r_ids), "rubric defines R# criteria", f"{len(r_ids)} found")
        report(bool(re.search(r"kill\s*condition", rubric.read_text(encoding="utf-8"), re.I)),
               "rubric declares kill conditions")

    # --- critiques: grounded and falsifiable --------------------------------
    for name in critiques:
        text = files[name].read_text(encoding="utf-8")
        cited = set(re.findall(r"\b([CAUFR]\d+)\b", text))
        ghost = cited - valid_ids
        report(bool(cited & valid_ids), f"{name}: objections cite real IDs",
               f"cited {len(cited & valid_ids)} valid" + (f", {len(ghost)} unknown: {sorted(ghost)}" if ghost else ""))
        report("FATAL-IF" in text, f"{name}: contains a FATAL-IF objection")

    # --- options: no self-ranking, axes declared ----------------------------
    for name in options:
        text = files[name].read_text(encoding="utf-8").lower()
        leaked = any(w in text for w in ("recommended option", "best option",
                                         "preferred option", "clear winner"))
        report(not leaked, f"{name}: no preference language",
               "contains ranking/preference wording" if leaked else "")

    # --- decision: concession + fatal resolutions ---------------------------
    dec = files.get("05_decision.md")
    if dec:
        dtext = dec.read_text(encoding="utf-8")
        report(bool(re.search(r"concession", dtext, re.I)),
               "decision contains a concession section")
        n_fatal = sum(files[c].read_text(encoding="utf-8").count("FATAL-IF") for c in critiques)
        n_resolved = len(re.findall(r"FATAL-IF", dtext))
        report(n_resolved >= 1 and n_resolved >= min(n_fatal, 1),
               "decision addresses FATAL-IF objections",
               f"{n_resolved} referenced of {n_fatal} raised")
        report(bool(re.search(r"revisit", dtext, re.I)),
               "decision states revisit conditions")

    # --- audit: engages the sealed lean -------------------------------------
    aud = files.get("06_audit.md")
    if aud:
        atext = aud.read_text(encoding="utf-8")
        report(bool(re.search(r"sealed[_ ]lean|00_sealed", atext, re.I)),
               "audit references the sealed lean")
        report(bool(re.search(r"match|differ|changed|same", atext, re.I)),
               "audit states whether decision matched the lean")

    print()
    print("RESULT:", "ALL PASS" if FAIL == 0 else "FAILURES PRESENT — fix before presenting")
    print("Reminder: this checker verifies structure only. Passing it does not")
    print("certify the reasoning; the Phase 6 self-audit against")
    print("references/theater_patterns.md is still required.")
    return FAIL


if __name__ == "__main__":
    if len(sys.argv) != 2 or not Path(sys.argv[1]).is_dir():
        print("usage: check_process.py <forge_dir>")
        sys.exit(2)
    sys.exit(main(Path(sys.argv[1])))