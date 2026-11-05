# Topical guidance

Read the row whose trigger matches your work. Root rules are in [AGENTS.md](../../AGENTS.md); the crate map and
commands are in [README.md](../../README.md). These guides hold engineering method and findings. They hold no game
design: rows marked *reference* carry findings from a sibling project (Runner) to inform choices in `design/`, not
rules. A pointer to a missing target is a defect: fix it in the change that finds it.

| Trigger | Read |
|---|---|
| Writing Rust | [rust-conventions.md](rust-conventions.md): absence, errors, configuration, API shape, threads |
| Writing or running tests | [tests.md](tests.md) |
| Writing UI code, or anything that draws to the terminal or reads keys | [tui-ratatui.md](tui-ratatui.md) |
| Drawing pictures or maps in block glyphs (octant, quadrant), cell aspect, fonts, captures; *reference* for readability and renderer defaults | [rendering.md](rendering.md) |
| Sound effects, ambience, music, audio devices, offline renders; *reference* for mixing, loudness and music | [audio.md](audio.md) |
| Writing or checking content files (TOML, PNG) and their loaders | [content.md](content.md) |
| Writing anything the player reads as fiction (intro, work orders, reports, dialogue, descriptions): narrative voice | [player-text.md](player-text.md) |
| Any game prototype work: a new slice or system, a scope proposal, a design question to the user | [playable-first.md](playable-first.md) |
| Making a portrait, scene or item picture with the image model; *reference*: prompting and judging pictures that read in block glyphs | [generated-art.md](generated-art.md) |
| Top-down place views; *reference*: findings from another project's lit map | [place-views.md](place-views.md) |
| UI affordances and player guidance; *reference*: findings from another project's playtests | [ui-guidance.md](ui-guidance.md) |
| Using or changing the octant/quadrant encoder, image fit, PNG capture | [crates/terminal-graphics/README.md](../../crates/terminal-graphics/README.md) |
| Drawing an ASCII-authored, lit tile map of a place | [crates/terminal-tilemap/README.md](../../crates/terminal-tilemap/README.md) |
| Playing sounds through the audio crate | [crates/terminal-audio/README.md](../../crates/terminal-audio/README.md) |
| High-cost design, or writing a plan | [discovery.md](discovery.md) |
| Designing or reviewing a boundary, owner or shared state | [architecture-laws.md](architecture-laws.md) and the `natural-extension` skill ([routing](../../.claude/ROUTING.md)) |
| Writing a packet, running parallel agents, committing beside other agents | [delegation.md](delegation.md) |
| Planning work with the user, reviewing agent output, a recurring workflow problem | [lessons.md](lessons.md) |
| A build, shell, git or terminal problem | [common-issues.md](../common-issues.md) |
| Starting a checkpoint, issue list or experience log | [templates](../templates/) |
| Checking what Ratatui or crossterm actually does | The locked versions: [Ratatui 0.30.2 API](https://docs.rs/ratatui/0.30.2/ratatui/), `cargo doc -p ratatui --open`, or the source under `~/.cargo/registry/src/*/ratatui-0.30.2/`. `Cargo.lock` is the version authority |

Tools: [tools/wt-capture.ps1](../../tools/wt-capture.ps1) captures a real Windows Terminal window;
[tools/art/generate.py](../../tools/art/generate.py) makes missing pictures from a prompts file.
