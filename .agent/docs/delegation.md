# Delegation

Root rules are in [AGENTS.md](../../AGENTS.md) "Work". Delegate bounded work when it saves more context than the
brief costs; otherwise work inline. Delegation is not required for every code change. The lead owns scope,
contracts, integration, verification and the checkpoint.

## What to delegate

- Work whose output is verbose: broad searches, log reads, build and test runs where only pass, fail and the
  failing lines matter. Prefer a read-only search agent for sweeps.
- Implementation slices with disjoint files and a settled contract, run in parallel.
- The leaf returns the verdict, the changed files and the failing assertions, never the transcript.

Discovery, decomposition, core design and adversarial review stay with the lead or go to the no-stake review roles
in [.claude/ROUTING.md](../../.claude/ROUTING.md). Leaves implement against a settled design; a deviation stops and
comes back.

## Packet

- Goal, scope (explicit files), non-goals, contract, document pointers, acceptance, evidence ([AGENTS.md](../../AGENTS.md)).
- One kind of work: mechanical (renames, signature fixes) or design work, never both.
- One surface per packet: one contract, or one screen, or the settings. A packet that changed settings, keys and
  four screens at once ran 1.5 hours, passed 250k tokens and left its gate undone.
- Model and effort: medium effort by default. High effort needs a reason in the packet (an open diagnosis, or a
  contract many packets build on). Integration, captures, test fixes and renames run at medium. Fix a vague packet
  instead of raising its effort.
- Evidence required: the build result and a named test run; for UI work, a window capture at readable zoom.
- Pointers name doc sections, not whole documents. If the work matches a trigger in [README.md](README.md), name
  that guide.
- Reading budget: the packet names the files with the line ranges or symbols to read. An agent reads a file whole
  only if it is under about 300 lines; otherwise by range or symbol. Bounded greps (counts, first matches), never
  full logs. Reads dominate cost: a token read early is paid again on every later request.
- Name where durable findings go (a guide, `common-issues.md`, ADR banners, code comments).
- A brief past about 40 lines shows that the design is not settled. Settle it first.
- Report: result, paths, evidence, deviations, blockers; 15 lines at most.
- No nested delegation. A middle agent hands back before its children finish, and their reports get lost.

## Cache, context and quota

Every request resends the whole context. Prices per million tokens measured from this repository's session
transcripts (Claude Opus, 2026-11-04): fresh input $4, cache read $0.20, cache write $8 (1-hour entries), output $20.
A written token costs as much as 40 cache reads. Spend and subscription quota follow these weights.

Measure a session with `python tools/token-report.py` (newest session; `--all` for every one): requests, cost by
token kind, peak and average context, cache rewrites and cache lifetime per agent.

Facts that shape every decision:

- Per agent: cache reads ≈ $0.20/M × requests × average context; cache writes ≈ $8/M × final context; output is
  thinking plus written code. Reads grow with the product of request count and context size.
- First fan-out session in this repository (12 agents, 712 requests, $51.65): cache reads 40 %, writes 33 %, output
  27 %. Big implementation agents made 75–127 requests at 140–170k average context and ended at 210–280k. The
  sibling project measured 53 % / 26 % / 21 % over 12,695 requests.
- Cache lifetime: the working assumption is 1 hour for the lead and 5 minutes for subagents. The harness setting
  `providers.cacheRetention: auto` wrote every entry in this repository's first session, lead and subagents alike,
  at the 1-hour rate (`cttl.ephemeral1h` in the transcripts), so subagents pay the 1-hour write premium (2× fresh
  input instead of 1.25×). An idle subagent is parked after `task.agentIdleTtlMs` (7 minutes).
- An expired cache is rewritten in full on the next request: resuming a 200k-token agent after expiry pays $1.60
  before it does anything.
- Subagent sessions become very inefficient past about 300k tokens: each request re-reads the whole prefix and the
  work per request does not grow.

Consequences:

- **Only the lead keeps context.** Its 1-hour cache makes accumulated context pay off. Subagents are disposable:
  one packet, one fresh agent, done. Durable results go to files, not to a subagent's memory.
- **Follow-ups only while warm.** Message a finished subagent only within about 5 minutes of its last request, on the
  same topic, with context under about 150k. Otherwise start a fresh agent briefed from durable sources (guide
  sections, commits, file ranges), never from the old transcript.
- **Size packets to finish well under 300k.** Aim for 100–200k. Past about 250k an agent finishes its step, commits,
  writes findings to the named durable places and reports what is left. Make packets small by scope (one surface),
  not by retiring agents early: a fresh agent pays about $2 to read back to 100k.
- **No waits longer than the cache.** A subagent tool call that runs past 5 minutes (a long build or test run, a
  blocking wait on another agent) expires its cache, and its next request rewrites everything. Keep subagent test
  runs in seconds ([tests.md](tests.md)); the lead runs long gates. The lead's own idle gaps stay under 1 hour.
- **Share prefixes when fanning out.** Siblings spawned together with identical leading text (the shared `context`
  block first, the per-task text last) can reuse each other's cached prefix. Spawn related packets in one batch;
  keep the shared block byte-identical; put everything that differs at the end.
- **Keep the prefix stable.** Root guidance (`AGENTS.md`, `CLAUDE.md`) and skill files load into every agent's
  prefix. Edit them between work sessions or at milestones, not while a fan-out runs, and keep them short: every
  agent pays for every line on every request.
- **Reports stay small.** A subagent's report enters the lead's long-lived context and is re-read for the rest of
  the session. 15 lines; artifacts by path.
- **Fewer requests per packet.** Reads scale with request count. Batch independent reads, greps and edits into one
  turn; create a new file with one write instead of many edits; fix all compiler errors from one `cargo check`
  before rebuilding; stop at acceptance without re-auditing. Aim for under 50 requests per packet.
- **Less ingest per agent.** Every token an agent takes in is written once at $8/M. The lead pastes the exact
  signatures, paths and line ranges it already knows; crate READMEs carry an API index so agents do not open source
  to learn an interface; quiet tool output (`cargo -q`, `--message-format short`, test `-q`).
- **Cheaper effort and models for mechanical work.** Output (thinking plus code) is about a quarter of spend.
  Scouts, captures, renames, integration and test fixes run at medium effort or on a cheaper model when the harness
  allows it (`task.enableEffort`, `task.perCallModel`, the `smol` model role; user settings).
- Split a packet that changes a shared contract and its users (for example settings, keys and several screens) into
  a contract packet and a users packet.

### Compaction (lead)

The lead's context grows across the session and is re-read on every request. At natural break points, tell the user
that compaction is a good idea now and why. Natural points: a milestone is committed and the checkpoint is updated;
the next work is a different topic; the lead's context has grown past a few hundred thousand tokens with mostly
finished work in it; or the user is about to pause for longer than the 1-hour cache. Update `.agent/checkpoint.md`
first, so the compacted or restarted session continues from it. Do not suggest compaction mid-integration or while
subagents still owe reports.

## Hard constraints

- One agent at a time uses a stateful tool (a terminal session, a device, a build in a shared tree).
- Concurrent implementation runs in separate git worktrees, one branch each. In the shared tree, leaves do not stage,
  commit or run Cargo.
- The integration owner changes shared manifests, lockfiles and guidance, and runs checks after integration.
- The lead is the only writer of the checkpoint and folds leaf findings into `common-issues.md`.
- One-off scratch scripts and message files carry the packet's name and are deleted when the packet ends.

## Parallel agents and git

- Give parallel packets disjoint files. `git commit -- <path>` commits the whole file, including another agent's
  uncommitted hunks.
- If two agents must touch one file, commit only your hunks through a temporary index (`git add -p` needs an
  interactive terminal):
  1. `GIT_INDEX_FILE=<scratch>/idx git read-tree HEAD`
  2. Write the file as HEAD plus only your hunks to a blob: `git hash-object -w --stdin < <file>`.
  3. `GIT_INDEX_FILE=<scratch>/idx git update-index --cacheinfo 100644,<sha>,<path>`
  4. Commit with the same `GIT_INDEX_FILE`.
  5. Run `git reset -q -- <committed paths>` with the real index. Without it the real index still holds the old
     content, and the next commit silently reverts yours.
- Before a merge in a shared tree, every agent commits, and tools that watch the tree stay idle.
- An agent stopped by a limit may resume later. Confirm its final report before launching a packet with
  overlapping scope.
- Review deliverables land on the shared branch before the user hears of them. No link points into a worktree.

## Destruction boundary

Never delete or overwrite anything the packet did not create. Report stray artifacts. Uncommitted work in the tree
belongs to the user.

## Review

- Check artifacts, not reports: compiler output, the test run, the diff against the acceptance.
- Compiling does not meet acceptance. The behaviour must run under a real producer.
- Check each commit's file list against the packet's scope.
- Review concurrent or hard-to-test code with a no-stake reviewer before building on it.
