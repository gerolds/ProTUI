---
name: handoff
description: Compact the current conversation into a handoff document for another agent to pick up.
argument-hint: "What will the next session be used for?"
---

Read [the shared execution contract](../../ROUTING.md) before applying this skill.


Write a handoff document summarising the current conversation so a fresh agent can continue the work. Use the repository's existing handoff path when the assignment owns it. Otherwise use a unique host-provided scratch path, report that path, and do not overwrite an existing handoff. Do not edit a lead-owned checkpoint from a leaf task.

Suggest the skills to be used, if any, by the next session.

Do not duplicate content already captured in other artifacts (PRDs, plans, ADRs, issues, commits, diffs). Reference them by path or URL instead.

If the user passed arguments, treat them as a description of what the next session will focus on and tailor the doc accordingly.
