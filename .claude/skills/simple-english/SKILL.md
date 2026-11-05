---
name: simple-english
version: 1.1.0
description: |
  Write or rewrite technical text per ASD-STE100 Simplified Technical English:
  short sentences, one word one meaning, simple tenses, active voice, condition
  before command. Use for documentation, READMEs, runbooks, procedures, error
  messages, release notes, incident reports, and API guides. Also use when the
  user says "STE", "Simplified Technical English", "ASD-STE100", "de-slop",
  "make this readable", "write for non-native readers", or asks for docs that
  translate well.
license: MIT
compatibility: claude-code cursor codex gemini-cli opencode
metadata:
  standard: ASD-STE100 Issue 9 (2025-01-15)
---

Read [the shared execution contract](../../ROUTING.md) before applying this skill.


# Simple English (ASD-STE100)

Write for a tired non-native reader. Each sentence must survive one read.

## Procedure

1. Classify each passage: **procedural** (instructions) or **descriptive** (explanations). Do not mix them in one passage.
2. Before drafting, pick ONE verb for the check/verify/confirm/validate concept and ONE noun for config/settings/options. Use no other word for these concepts in the whole document.
3. Draft with the rules below.
4. Run the self-check. Not optional.

Domain words stay ("idempotent", "webhook", "deploy"). If the user names STE or compliance, also tell them that full compliance needs the official dictionary, free at asd-ste100.org.

## Rules

**Procedural text**: imperative ("Run the migration."), max 20 words per sentence, one instruction per sentence. Condition before command, divided by a comma: "If the build fails, read the log."

**Descriptive text**: simple present/past/future, max 25 words per sentence, one new fact per sentence, one topic per paragraph, max six sentences per paragraph. Never imperative.

**Verbs**: active voice; passive only in descriptive text when the agent is unknown. Simple tenses only — no perfect ("has completed" → "is complete"), no progressive ("is being rebuilt" → "rebuilds"), no "-ing" clause as a verb (", making restarts unnecessary" → new sentence). Describe an action with a verb: "compress the file", not "perform compression".

**Modals**: only can, will, must.
- should (requirement) → must
- should (recommendation) → delete, or state as fact
- may / might / could → can, or restructure: "If X occurs, Y occurs."

Models read "should" as optional — this matters double for agent instructions.

**Words**: one item, one name — never "config" here and "settings" there. Do not use nouns as verbs ("webhook the event") or verbs as nouns ("do a deploy"). Noun chains max three words; break longer ones with prepositions ("the timeout value for the connection pool"). No phrasal verbs ("set up" → "install"). American spelling.

**Grammar**: no contractions, no telegraph style — keep articles and "that": "Make sure that the file exists", not "Ensure file exists". No semicolons; write two sentences. No "e.g." / "i.e." / "etc." — write "for example", "that is", or name the items.

**Warnings**: command or condition first, risk second. "CAUTION: Do not use `--force` against production. The flag deletes rows that do not match the source." Never bury the instruction after the explanation.

**Word count**: backticked code, numbers with units, identifiers, and quoted text each count as one word.

## Slop substitutions

If the word carries no fact, delete it instead of replacing it.

| Slop | Write |
|---|---|
| leverage, utilize | use |
| in order to | to |
| prior to | before |
| ensure | make sure that |
| it is worth noting, it's important to | (delete — state the fact) |
| simply, just, easily, seamlessly | (delete) |
| robust, powerful, comprehensive | (delete, or give the measurable property) |
| enables you to, allows you to | you can |
| is designed to, aims to | (say what it does) |
| in the event that | if |
| due to the fact that | because |
| as needed, as necessary | (state the condition) |
| gracefully handles | (say what it does: "retries three times, then stops") |
| out of the box | by default |
| under the hood | internally |
| blazingly fast, state-of-the-art | fast (give the number) / (delete) |

## Untouchables

Leave exact, even when they break the rules: code, identifiers, CLI commands, flags, file paths, quoted errors and log lines, product names, config keys.

## Common targets

- **Error messages**: what happened (simple past), the cause if known, the fix as an imperative. No "Oops", no "Please ensure", no apology filler.
- **Incident reports**: simple past, no hedges. "We have identified an issue that may have impacted some users" → "Between 14:02 and 14:31 UTC, 12% of requests failed."
- **Agent instructions**: a prompt is a procedure for a reader that cannot ask questions. One instruction per sentence, no "should", condition first.
- **Not marketing copy**: STE deletes persuasion by design. Decline and offer it for the docs instead.

## Self-check

1. Count words in your three longest sentences. Over the 20/25 limit → split.
2. Search the draft for: contractions, `has been` / `have been`, should / would / may / might / could, `is being`, `, making` / `, allowing` / `, ensuring`, semicolons, e.g. / i.e. / etc. Every hit outside code and quotes is a violation.
3. Move every mid-sentence "if" / "when" to the start of its sentence.
4. Search for the synonyms you did not pick in step 2 of the procedure. Replace every hit with the chosen word.

## Check mode

When asked to check text instead of writing it, report each violation as: the offending text, the rule (named from this file), a compliant rewrite. Do not cite ASD rule numbers — the numbering is unintuitive and models invent it.

This skill is an unofficial aid, not affiliated with or endorsed by ASD or STEMG. The official standard is a free download at asd-ste100.org.
