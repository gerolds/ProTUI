// ADR: One contract-test binary for the crate: one link step, modules per area.
#[path = "terminal_audio/engine.rs"]
mod engine;
#[path = "terminal_audio/layer.rs"]
mod layer;
#[path = "terminal_audio/manifest.rs"]
mod manifest;
#[path = "terminal_audio/reload.rs"]
mod reload;
#[path = "terminal_audio/variation.rs"]
mod variation;
#[path = "terminal_audio/wav.rs"]
mod wav;
