---
name: docs
description: How we write and maintain documentation. Use whenever you create, edit, review, or look for documentation of any kind — pages, READMEs, code comments, decision records. Also use when someone asks a question that documentation should have answered, or when you notice documentation that breaks these rules.
---

Read [the shared execution contract](../../ROUTING.md) before applying this skill.


# How we document

Ten rules, each with its reason, so edge cases can be judged.

## Where things live

**1. Write it where it changes.**
A fact that changes together with an artifact — code, asset, content file,
config — lives with that artifact: a comment, a note on it, a README in
its folder. The docs repo holds only what no artifact can hold: reasons,
decisions, workflows that cross folders and disciplines. Some things
have no artifact at all — a convention, a process; then the page *is*
the artifact, owned and checked like one. Test: would updating the fact
and the artifact be one change? Then they're one place.

**2. Say everything once.**
Before writing, search. If it exists, link to it. If it's almost right, fix
it there. Two copies of a fact means one is wrong and nobody knows which.
Views a machine derives from one source — generated reference,
transcluded snippets — are not copies: don't hand-edit them, fix
the source.

**3. Keep references stable.**
Use repository-relative Markdown links to the canonical page and heading.
Use stable IDs when the repository already has them or a decision record
needs one. Do not require a new ID registry for ordinary documentation.
When moving a page, migrate its callers in the same change.

## What's true

**4. Every claim points at its source.**
A file, a commit, a decision record, a named person, or a thing you can try
yourself. A claim with no source is an opinion — label it as one or leave
it out. Confidence in the prose is not a source.

**5. Say what you don't know.**
"Nobody remembers why" and "unverified, inherited from the old system" are
good sentences. A plausible guess dressed as an explanation is the worst
thing a doc can contain, because nobody will ever question it.

**6. Separate fact, proposal and approval.**
State unverified claims and proposed decisions explicitly. Agents can maintain
current engineering guidance from inspected evidence; that does not imply
human approval of design. Claim human review only when the conversation or
record contains it. Do not create a status/owner ceremony unless the repository
uses one. A draft is not a completed deliverable.

**7. Distinguish accepted decision records from current guidance.**
An explicitly accepted historical decision records what was chosen, the
rejected options, the reasons and the unknowns at that time. Do not rewrite
its history: supersede it with a new accepted record when needed.
Current guidance, README commands, checkpoints and constraints are mutable:
replace stale claims in place, remove completed status, and migrate links.
Do not preserve stale instructions by appending corrections or importing
old decision records into a new project. Git holds history.
Agents can decide reversible engineering choices within their assignment;
escalate changes to player experience or costly contracts per root guidance.

## How to write

**8. Fill the form, don't write an essay.**
Each page type is a few fixed fields — an idiom is *do this / not that /
because / checked by*, a pitfall is *symptom / cause / fix*. Copy a maintained page of
the same type and match it. If you're writing
sentences that connect other sentences, you're decorating; stop.
An entry is one filled form; entries of one form about one subject share
a file; decisions always get a file each. All forms filled, and how they
group: [examples/forms.md](examples/forms.md).

**9. Write for one reader with one problem.**
Before writing, name them: what they just hit, what they wrongly believe,
what they can do when done. If you can't name the reader, you're writing
for the shelf. Shorter is better; a link is better than a recap; nothing
is better than filler. And write plain English: common words, short
sentences, one meaning per sentence. Many readers use English as a
second language — the simplest phrasing that keeps the meaning is the
correct one, not a style choice.

**10. Write when asked, test by use.**
A question answered twice becomes a page — the second answer is the link.
Don't document ahead of need; an explicit missing prerequisite beats a polished page
nobody asked for. Do not ship requested work as a stub. To check a page, don't reread it: give it to someone
(or a fresh agent) with the real task and no other help. Where they fail
is what to fix.

## When you're unsure

Rules 1–3 tell you where. Rules 4–7 tell you what may be claimed.
Rules 8–10 tell you how much to write — the answer is usually less.
If two rules seem to conflict, the smaller change wins: fix in place,
link instead of copy, label uncertainty instead of guessing. Ask only when sources cannot settle it.

## See it

One screen each. Read the one next to the rule you're about to apply.

- [examples/forms.md](examples/forms.md) — templates to copy, filled examples in the appendix. (rule 8)
- [examples/before-after.md](examples/before-after.md) — one bad page and its rewrite. (rules 4, 5, 8, 9)
- [examples/where-things-live.md](examples/where-things-live.md) — the placement ladder, worked through for code and for art. (rules 1, 2)
- [examples/readme.md](examples/readme.md) — a good module README. (rules 1, 9)

## Verdicts

Decisions we've made. No discussion here — the rule behind each is above.

- Documenting everything — **bad.** Write when asked twice.
- Long = thorough — **bad.** Length is where errors and theater hide.
- A guess that sounds like an explanation — **worst.** Say "we don't know."
- "It's worth noting…", hedges on every claim — **bad.** Anchor or cut.
- Comments that restate the signature — **bad.** Comment the contract, units, lifetime, gotcha — or nothing.
- "Self-documenting code" — **half.** Names replace *what*-comments. Nothing in code holds *why*.
- Copying a fact "so the reader doesn't have to click" — **bad.** That copy is tomorrow's contradiction.
- FAQ pages — **bad.** A question asked twice earns a real page, not a landfill entry.
- Big documentation sprints and backfills — **bad.** Update docs when you touch the code.
- Explicit uncertainty — **good.** Honest gaps beat false coverage; they do not replace requested implementation.
- Deleting a page — **good.** Deletion is maintenance.
- Many small files — **good.** Easy to delete, move, and review; big mixed files hide things.
- Project instructions that could fit any project — **suspect.** Ground them in local code. A deliberately reusable skill may explain a portable mechanism.
- Editing an accepted decision record — **bad.** Write a new one that points at the old.
- Invented approval — **bad.** Distinguish inspected facts, engineering decisions and human-accepted design.
- Screenshots as API documentation — **bad.** Prefer text. Screenshots as visual evidence — **good** when the actual rendered result is the acceptance criterion.
- Diagrams — **good** when structure is the point, **bad** as decoration.
- A disconnected prose style guide — **bad.** Use the local exemplars and the `simple-english` skill when writing clarity is the task.
- A rare word where a common one works — **bad.** Not everyone reads English natively; simple is the house accent.
- Syntax only one renderer understands — **bad.** Plain markdown, front matter, standard links. The corpus must outlive its viewer.
- Debating the perfect structure before writing — **bad.** Relative links and git moves make reorganizing cheap; write.
- Freestyle thinking in working notes (`scratch/`) — **good.** The forms govern docs, not thinking. Just never publish the dig site.
- Docs referencing `scratch/` — **bad.** A plan records thinking at a date, not the system today. Promote it into a page or record, or leave it unreferenced.
- Hand-editing an auto-maintained view — **bad.** The machine will overwrite you; fix the source.
