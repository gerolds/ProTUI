# Rust conventions

Rules for shipped Rust code. Root rules are in [AGENTS.md](../../AGENTS.md) "Code".

## Absence, errors and configuration

Code that breaks these rules is a defect, whatever the packet said.

- A value that must be present is not an `Option`. Check it where it enters (content loading, save loading,
  settings, a constructor) and give the rest of the code a type that cannot be empty.
- `None` never carries meaning by itself. `set(foo, None)` is a defect: write `clear(foo)` beside `set(foo, value)`.
  Where a value may be absent, pair it with an enum variant that says what happens.
- Never repair bad configuration at runtime: no substitute defaults, no quiet early returns, no "if missing, use X".
  Refuse it with a typed error.
- `unwrap`, `expect` and `panic!` state a broken invariant, never a possible input. `expect` names the invariant. A
  missing file, a bad save or a bad key is an `Err`, not a panic.
- Errors are enums. A caller matches on a variant; no caller reads an error message. A crate with no dependencies
  implements `std::fmt::Display` by hand.
- Return `Result` across boundaries to other systems (files, the terminal, deserialization, third-party crates).
  Convert their errors into our enums there.
- Where possible, design so that no validation is needed: enums over flags, newtypes over bare numbers and strings.

## API shape

- An operation that can fail returns `Result<T, E>` with a typed `E`. An operation whose only failure is "not
  there" returns `Option<T>`.
- Resolve ambiguity with enums, traits or function parameters, never with stringly-typed APIs.

## Design defaults

- Deep modules: small public surfaces that hide a lot of capability. Keep items private or `pub(crate)` until
  another crate needs them.
- Composition over trait hierarchies.
- Immutability and pure functions; no `static mut`, no global `Mutex` holding game state.
- Early returns for invalid states over defensive layering.
- Plain data flow: locals, return values, small tuples and structs. Skip traits with one implementation, injected
  strategies and helper types that do not compress meaning.
- Treat allocation rate, frame budget and worst-case latency as features; design APIs that make them visible.

## Unsafe code

The workspace forbids `unsafe`. A change that needs it goes to the lead first. If approved, assert every boundary
condition around it: pointer ranges and lengths, indices, buffer sizes, layouts shared with native code.

## Code patterns

- Private fields with methods over public mutable fields. Public fields only on plain data with no invariant.
- Hot paths: no allocation per frame where a reused buffer works, no boxing, no repeated lookups.
- Before you add a pattern, read nearby code and follow the dominant local convention.
- Make init idempotent instead of guarding it with a `ready` flag.
- When call order matters, make it explicit in one orchestrating function.
- Format with `cargo fmt`. Clippy warnings are errors in `verify.cmd`. An `#[allow(...)]` names its reason in a
  comment beside it.

## Events

- Rules report what happened as returned values: a command returns its result and a list of typed events. No
  central publish/subscribe bus, no callbacks into the rules.
- Name events with a noun or past-tense verb (`ItemAdded`).

## Threads

- One thread runs the game loop and owns all game state. Work moved to another thread (decoding, file writes) sends
  its result back as a message; it never touches game state. The audio thread is separate ([audio.md](audio.md)).
- No async runtime unless a dependency forces one; the comment beside it names why.

## Naming and comments

- Follow the Rust API guidelines. Names carry domain meaning; no abbreviations except `id`, `config`, `info`.
- `try_*` only for an operation that can fail when a non-failing twin exists.
- Default to no comments. Add one only when the *why* is not obvious. Doc comments (`///`) say what an item does
  and what the caller gets.
- Each Rust or script file starts with 1–3 lines `// ADR: <decision> [— <one-clause why>]`: current decisions, not
  history. Rewrite and consolidate rather than append; delete a replaced decision.

## Tool code versus shipped code

- Tool code (importers, inspectors, bench and render commands) consumes and authors runtime types. It may depend on
  runtime crates; runtime crates never depend on tool code.
- No game logic in tool code, not even "just for testing". Call runtime validation; do not duplicate it.
- When you change a data type, a public API or a serialized shape, check the tool code that uses it.
