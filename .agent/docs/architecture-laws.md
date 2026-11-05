# Architecture laws

Seed laws for reviewing a boundary, an owner or shared state. They are a checklist, not a framework to build. The
`natural-extension` skill ([routing](../../.claude/ROUTING.md)) describes how to test a change against the laws
the code obeys. Root rules: [AGENTS.md](../../AGENTS.md) "Design" and "Code".

## Laws and anomalies

- A law is something the code currently obeys, extracted from working code, not decreed. It has a reason, an
  example (file and item) and a counter-example.
- A change that breaks a law ships only with a recorded reason and a plan: fix now, needs a design turn, needs a
  decision.
- When a law turns out wrong, amend it in place and say what the first version missed.
- Where written contracts say something the code has never done, record a contradiction, not a law.

## Seed laws

### Rules stay independent of the terminal

Game rules take commands and return results and events. They do not read keys, draw cells or play sound. Rendering
and audio read state; they never change it.

### One owner per feature

A feature is owned by one module. Other code asks the owner; it never reaches a peer's internals. One place owns
state, lifetime and ordering, so "who decides this?" has one answer.

### Hold the owner, not the instance

Do not keep a reference or index to a created object across frames unless the owner guarantees it stays valid (for
example indices that are never removed). Ask the owner when you need the answer. A stored answer goes stale.

- The owner holds authored and created instances alike.
- The owner refuses rather than chooses. Zero candidates and two candidates are both authoring mistakes; report them.
- Validate at start-up, where an author sees the problem before a player does.

### Dependencies point one way

A lower module is told who something belongs to (an ID) and works out the rest itself. It is never handed a verdict
such as a bare `bool`, and never reaches upward.

### Failure is typed and refuses rather than repairs

Return `Result` with a typed error, or `Option` for "not there" ([rust-conventions.md](rust-conventions.md)). Never
create a missing piece or quietly fix it. A silent repair moves the bug to a place nobody can trace it from.

### One authority per piece of state

A piece of state has one writer. Uncoordinated writers do not average; whichever runs last wins, silently.

### State goes into the store that outlives its cause

An event may trigger a write; it never holds one. Before writing, name the store and check that it outlives the
trigger: a frame ends, a screen closes, a session ends, a save outlives them.

## Design pitfalls

- **Invented parts.** A design that reads as consistent but the user rejects as "not real" describes stock parts
  (registries, tiers, core-plus-plugin) instead of the mechanism. Signs: the same invented parts renamed across
  turns; a choice between two arrangements of the same invented design; a design that would read the same for any
  domain. Fix: map every noun to a noun or verb in the user's own description and delete the rest. Build one
  concrete vertical first; extract shared code only when two cases are literally duplicated.
- **"Has happened" and "what happened" are two facts.** A default enum value is not success.
- **"Unbound" and "available" are two facts.** An object released but promised back is excluded from every
  hand-out by structure, not by convention.
- **A restore replaces live state; it never merges.** Release live state through the teardown path, then apply the
  records. Test a restore over an advanced live state.
- **A cache keyed by an entity must not outlive the entity.** Drop its entry when the entity goes.
- **A submit loop that stops on the first refusal treats every refusal as back-pressure.** Return a typed reason and
  stop only on back-pressure.
- **Clone before you mutate shared loaded data.** Runtime edits otherwise leak into the next use or get saved.
- **State that is not saved is lost on reload.** Keep a rebuild path from the saved source of truth.
- **Every wait is a source of lag.** List every wait in a pipeline with its bound.
