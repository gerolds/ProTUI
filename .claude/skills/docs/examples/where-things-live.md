# Where things live

Ask these questions in order. The first yes gives the smallest home for a fact.
This is a placement ladder, not a request to create every kind of document.

1. **Can the artifact explain itself?** Rename an ambiguous parameter to include
   units. Prefer a working example over prose that repeats its setup.
2. **Does the fact change with one artifact?** Put its contract, units, lifetime or
   surprising constraint in a doc comment or the source/config beside it.
3. **Is it about one module or folder?** Put purpose, entry routes and specific
   surprises in that module's README. Pin provenance/version beside adopted code.
4. **Does it span artifacts or have no other body?** Put a cross-module workflow,
   reason or current constraint in the repository's existing guidance location.

Use the local repository layout, not a copied engine's `Assets/` or `Packages/`
conventions. Ordinary links name actual files and headings. IDs help only when
there is an existing ID-based corpus or a durable decision identity.

## Working notes are not current instructions

Use the host scratch location or the assigned plan path for exploratory thinking.
A scratch plan may cite current docs; do not make lasting instructions depend on
an ephemeral plan. Promote durable findings into their canonical artifact and
remove obsolete scratch files in scope. Do not preserve stale status for history;
Git holds history. Preserve user-owned source material and unrelated working notes.

An in-file decision note explains why this file is shaped this way. Promote it to
a cross-module record only when readers who never open this file need the reason.
Do not create an automatic-consolidation system merely because a source example
mentioned one.

## Accepted records versus mutable guidance

An explicitly accepted historical decision preserves its options and unknowns at
that moment. Supersede it when the decision changes. A README, root guidance page,
checkpoint or current law is not an immutable ADR: replace stale instructions in
place and migrate links. Never imply that a generated proposal is human-approved.

## One subject, three homes

These are illustrative facts, not installed APIs of this repository:

- A pixel constructor accepts linear light, not sRGB: the constructor's doc comment.
- A conversion cache is invalidated by cell aspect changes: the module README and
  implementation contract, each holding its own level rather than copied prose.
- A project accepts octants despite a font requirement: the current project choice,
  with the accepted tradeoff and the evidence used to make it, if requested.

## The same ladder for art

- A texture's colour profile and channel layout live with the source/export recipe.
- A folder's permitted sources and export command live in its README.
- A chosen art direction belongs to design, not to a reusable encoder's guide.
- A review workflow with no other artifact lives as a workflow page.

The page itself is the artifact when it defines a workflow. Ground its claims in
observable evidence and current authority; do not invent an owner/status registry.
