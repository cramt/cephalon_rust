# Relic Reward Overlay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In-game overlay that shows platinum prices under each relic reward card, built on Freya + winit (Orbolay's tech), fed by the existing core pipeline.

**Architecture:** `core` stays frontend-agnostic and emits an `Event` stream (screen opened / rewards resolved / screen closed) over a tokio mpsc channel; card positions come from a shared pure-math `geometry` module. A new `overlay` crate runs a transparent, undecorated, click-through, monitor-sized Freya window that is shown only while a reward screen is up. `cli` remains the headless consumer.

**Tech Stack:** Rust (workspace: core/cli/overlay), Freya `0.4.0-rc.24` (builder API — NOT rsx!), winit 0.30, display-info, tokio, ocrs 0.12 + rten 0.24, xcap, Nix flake devshell on NixOS.

**Spec:** `docs/superpowers/specs/2026-07-09-relic-overlay-design.md`

## Global Constraints

- **Frontend-agnostic core:** `core` must never depend on winit, freya, display-info, or anything display-server-related. Its contract: `Engine::new(cache_path)`, `Engine::run(sender)` emitting `Event`, and the pure `geometry` module.
- **Freya 0.4 is NOT Freya 0.3.** No `rsx!`, no Dioxus, no `use_signal`/`use_effect`/`use_coroutine`. Use the builder API (`rect().width(Size::fill()).child(...)`), struct components implementing `Component` with `derive(PartialEq)`, `use_state`/`State<T>` (Copy), `use_hook` for one-time setup, `spawn_forever` for detached async, `use_side_effect` for reactive effects, `Platform::get().with_window(None, |w| ...)` for raw winit window access.
- **Pin freya exactly:** `freya = "=0.4.0-rc.24"` (pre-release; API churns between rcs).
- **All commands run inside the Nix devshell** (`nix develop`, or direnv if set up). The shellHook exports `CACHE_PATH=./app_cache` and `LD_LIBRARY_PATH`.
- **OCR-dependent tests are slow without optimization.** After Task 1 adds `[profile.dev.package."*"] opt-level = 3`, plain `cargo nextest run` is fine. If a test involving `ocr` still crawls, fall back to `cargo nextest run --release`.
- **Tests that touch item data hit warframe.market** on first run and cache into `CACHE_PATH` (existing behavior; keep it).
- **Commits:** small and frequent, lowercase casual style matching repo history (`git log --oneline`). NEVER add Co-Authored-By lines.
- **Edition stays 2021** for all crates (workspace convention).
- No `freya-testing` UI tests in this plan: label layout is a trivial function of the unit-tested `geometry` module, and pixels are verified end-to-end in Task 8. Don't add them.

---

### Task 1: Dependency refresh + dev-profile optimization

The workspace deps are ~1 year stale. Update everything before new code so upgrade breakage stays isolated from feature breakage. Research verdict on OCR: **keep ocrs** (actively maintained mid-2026), bump `ocrs` 0.10.3 → 0.12.x with its lockstep `rten` 0.18 → 0.24. The `.rten` model files and `include_bytes!(env!(...))` embedding keep working unchanged.

**Files:**
- Modify: `Cargo.toml` (workspace root — add profile section)
- Modify: `core/Cargo.toml`, `cli/Cargo.toml` (version bumps)
- Modify: `flake.lock` (via `nix flake update`)
- Possibly modify: `core/src/**` call sites broken by upgrades (notably `xcap` API changes)

**Interfaces:**
- Consumes: nothing (first task)
- Produces: a workspace where `cargo nextest run` passes on current-2026 deps; later tasks assume `ocrs = "0.12"`, `rten = "0.24"`, and fast debug-mode OCR via the profile tweak.

- [ ] **Step 1: Update the flake inputs**

```bash
nix flake update
git add flake.lock
```

Then re-enter the devshell (`exit` + `nix develop`, or `direnv reload`) so the newer stable toolchain from rust-overlay is active. Freya rc.24 needs a recent stable (1.95+); verify with `rustc --version`.

- [ ] **Step 2: Bump OCR deps explicitly, everything else to latest**

In `core/Cargo.toml` set:

```toml
ocrs = "0.12"
rten = "0.24"
```

Then upgrade the rest (cargo-edit is in the devshell):

```bash
cargo upgrade --incompatible
cargo update
```

- [ ] **Step 3: Add the dev-profile optimization for dependencies**

In the workspace root `Cargo.toml`, append:

```toml
[profile.dev.package."*"]
opt-level = 3
```

This optimizes dependencies (rten's inference kernels — where OCR time goes) while our own crates keep fast debug compiles. This kills the "have to run tests in release mode" problem and the README TODO about unusable debug builds.

- [ ] **Step 4: Fix compile breakage**

```bash
cargo check --workspace
```

Expected breakage spots, in likelihood order:
- `xcap` (0.6 → newer): `Window::all()`, `.title()`, `.capture_image()` signatures may have changed (e.g. `title()` returning `Result<String>` instead of `Option`-ish). Adapt call sites in `core/src/lib.rs` keeping identical behavior (find window titled `"Warframe"`).
- `ocrs`/`rten`: per research there are no breaking changes for our usage (`OcrEngine::new`, `Model::load_static_slice`, `ImageSource::from_bytes`, `prepare_input`, `get_text` in `core/src/ocr.rs`). If `Model::load_static_slice` moved/renamed, check the rten 0.24 docs for the static-slice loader and adapt.
- `config`, `reqwest-middleware`, `thiserror`, `memoize`: minor signature drift at most.

Fix until `cargo check --workspace` is clean. Do NOT refactor anything beyond what compiles — behavior-preserving fixes only.

- [ ] **Step 5: Run the test suite**

```bash
cargo nextest run
```

Expected: all existing tests pass (the 6 relic-screen OCR tests in `core/src/relic_screen_parser.rs` plus any others). First run downloads item data from warframe.market into `./app_cache` — needs network. If OCR tests are still unbearably slow, use `cargo nextest run --release` and note it.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "big deps update + optimize deps in dev profile"
```

---

### Task 2: Flake prep + overlay crate skeleton + COSMIC stacking spike

**This task gates the whole approach.** Build a minimal transparent Freya window with a colored box, run it over borderless Warframe on COSMIC, and verify: renders on top, clicks pass through, and `set_visible` re-mapping puts it back on top. If stacking fails, STOP and escalate to the user — the documented fallback is swapping the windowing layer to wlr-layer-shell (see spec Risks §1), which changes Tasks 7–8.

**Files:**
- Modify: `Cargo.toml` (workspace root — add `"overlay"` to members)
- Modify: `flake.nix` (Skia/Freya native deps)
- Create: `overlay/Cargo.toml`
- Create: `overlay/src/main.rs` (spike version — Task 7 replaces its body)

**Interfaces:**
- Consumes: nothing from other tasks
- Produces: `overlay` crate that compiles and launches; flake devshell able to build skia-safe; a go/no-go verdict on the window approach. Task 7 keeps the launch-config code from this spike.

- [ ] **Step 1: Add native deps to the flake**

In `flake.nix`, extend `commonArgs.buildInputs` (skia-safe needs font libs; winit-wayland needs libxkbcommon):

```nix
buildInputs = with pkgs; [
  openssl
  libGL
  wayland
  xorg.libxcb
  libgbm
  pipewire
  # freya/skia
  fontconfig
  freetype
  expat
  libxkbcommon
];
```

The devshell's `LD_LIBRARY_PATH` shellHook already derives from `commonArgs.buildInputs`, so these flow through automatically. Re-enter the devshell after editing.

Note: skia-safe's build script downloads prebuilt Skia binaries — fine in the devshell (network available), and the sandboxed `nix build` package of the overlay is explicitly deferred (spec Risks §2). If the skia build later complains about missing `clang`/`python3`, add them to `nativeBuildInputs`.

- [ ] **Step 2: Create the overlay crate**

Workspace root `Cargo.toml`:

```toml
[workspace]
members = [ "cli", "core", "overlay" ]
resolver = "2"
```

`overlay/Cargo.toml`:

```toml
[package]
name = "cephalon_rust_overlay"
version = "0.1.0"
edition = "2021"

[dependencies]
freya = "=0.4.0-rc.24"
winit = { version = "0.30", features = ["wayland", "x11", "rwh_06"] }
display-info = "0.5"
futures-timer = "3"
cephalon_rust_core = { path = "../core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
config = "0.15"
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
```

(`futures-timer` because Freya's `spawn_forever` executor is not a tokio runtime — `tokio::time::sleep` would panic there. Engine-side timers are fine; they run on our own tokio runtime thread.)

- [ ] **Step 3: Write the spike `overlay/src/main.rs`**

```rust
use std::time::Duration;

use freya::prelude::*;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::WindowLevel,
};

fn primary_display() -> display_info::DisplayInfo {
    let displays = display_info::DisplayInfo::all().unwrap_or_default();
    displays
        .iter()
        .find(|d| d.is_primary)
        .or(displays.first())
        .expect("no displays found")
        .clone()
}

fn main() {
    let display = primary_display();
    // +1/-1 px fudge: transparent windows at exact monitor size go black (Orbolay's workaround)
    let size = PhysicalSize::new(
        (display.width + 1) as f64 * display.scale_factor as f64,
        (display.height - 1) as f64 * display.scale_factor as f64,
    );
    let position = PhysicalPosition::new(display.x, display.y);

    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_title("cephalon")
                .with_decorations(false)
                .with_transparency(true)
                .with_background(Color::TRANSPARENT)
                .with_window_attributes(move |mut w, _event_loop| {
                    w = w
                        .with_inner_size(size)
                        .with_resizable(false)
                        .with_window_level(WindowLevel::AlwaysOnTop)
                        .with_position(position);

                    #[cfg(target_os = "linux")]
                    {
                        use winit::platform::wayland::WindowAttributesExtWayland;
                        w = WindowAttributesExtWayland::with_name(w, "cephalon", "cephalon");
                    }

                    w
                }),
        ),
    );
}

fn app() -> impl IntoElement {
    use_hook(|| {
        // click-through: empty input region on wayland
        Platform::get().with_window(None, |w| {
            let _ = w.set_cursor_hittest(false);
        });
        // exercise the show/hide cycle the real overlay will rely on
        spawn_forever(async move {
            let mut visible = true;
            loop {
                futures_timer::Delay::new(Duration::from_secs(5)).await;
                visible = !visible;
                Platform::get().with_window(None, move |w| w.set_visible(visible));
            }
        });
    });

    rect()
        .position(Position::new_absolute().top(200.).left(200.))
        .width(Size::px(400.))
        .height(Size::px(120.))
        .corner_radius(CornerRadius::new_all(16.))
        .background(Color::from_rgb(220, 60, 60))
        .center()
        .child(
            label()
                .font_size(32.)
                .color(Color::WHITE)
                .text("CEPHALON SPIKE"),
        )
}
```

- [ ] **Step 4: Build and run it bare (no game)**

```bash
cargo build -p cephalon_rust_overlay
cargo run -p cephalon_rust_overlay
```

Expected: first build takes a while (skia). A red box labeled "CEPHALON SPIKE" floats at (200,200) with fully transparent surroundings; it vanishes and reappears every 5s; you can click *through* both the box and the transparent area onto whatever is behind. If the whole screen is black instead of transparent, the compositor didn't get an alpha surface — check `with_transparency(true)` ordering and the +1px size fudge.

- [ ] **Step 5: MANUAL GATE — verify over Warframe (needs the user)**

This step requires the human: launch Warframe (borderless), then `cargo run -p cephalon_rust_overlay`, then focus Warframe again. Verify and record in the commit message:

1. Box renders above the game while the game has focus.
2. After a hide/show cycle (5s), the box re-appears **above** the game, not behind it.
3. Mouse input goes to the game everywhere, including through the box.

If (1) or (2) fails: STOP. Report to the user; the fallback is layer-shell windowing per the spec. Do not proceed to Task 7 without a decision.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "overlay crate skeleton + cosmic stacking spike (verified over warframe)"
```

---

### Task 3: Extract card geometry into `core::geometry`

Pure refactor with pinned-value tests. The card-position math currently lives inline in `parse_relic_screen`; both the parser (cropping) and the overlay (label placement) need it.

**Files:**
- Create: `core/src/geometry.rs`
- Modify: `core/src/lib.rs` (add `pub mod geometry;`)
- Modify: `core/src/relic_screen_parser.rs:54-75` (use the module)
- Test: inline `#[cfg(test)]` in `core/src/geometry.rs`

**Interfaces:**
- Consumes: nothing
- Produces: `pub struct CardRegion { pub x: u32, pub width: u32, pub text_bottom: u32, pub line_height: u32 }` and `pub fn reward_card_regions(screen_width: u32, screen_height: u32, count: usize) -> Vec<CardRegion>` in `cephalon_rust_core::geometry`. Task 7 positions labels with it.

- [ ] **Step 1: Write the failing tests**

Create `core/src/geometry.rs` with only the tests (types referenced don't exist yet):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_cards_1080p() {
        let r = reward_card_regions(1920, 1080, 4);
        assert_eq!(r.iter().map(|c| c.x).collect::<Vec<_>>(), vec![474, 717, 960, 1203]);
        assert!(r.iter().all(|c| c.width == 243));
        assert!(r.iter().all(|c| c.text_bottom == 460));
        assert!(r.iter().all(|c| c.line_height == 24));
    }

    #[test]
    fn four_cards_1440p_scales() {
        let r = reward_card_regions(2560, 1440, 4);
        assert_eq!(r.iter().map(|c| c.x).collect::<Vec<_>>(), vec![632, 956, 1280, 1604]);
        assert!(r.iter().all(|c| c.width == 324));
        assert!(r.iter().all(|c| c.text_bottom == 613));
        assert!(r.iter().all(|c| c.line_height == 32));
    }

    // NOTE: the 3-card order is intentionally non-monotonic — it mirrors the slot order
    // the original parser used (left, right, middle). Preserved verbatim; both parser
    // crops and overlay labels use the same slot->position mapping so they stay consistent.
    #[test]
    fn three_cards_preserves_original_slot_order() {
        let r = reward_card_regions(1920, 1080, 3);
        assert_eq!(r.iter().map(|c| c.x).collect::<Vec<_>>(), vec![596, 1081, 839]);
    }

    #[test]
    fn two_and_one_cards() {
        let two = reward_card_regions(1920, 1080, 2);
        assert_eq!(two.iter().map(|c| c.x).collect::<Vec<_>>(), vec![717, 960]);
        let one = reward_card_regions(1920, 1080, 1);
        assert_eq!(one.iter().map(|c| c.x).collect::<Vec<_>>(), vec![839]);
        assert!(reward_card_regions(1920, 1080, 5).is_empty());
        assert!(reward_card_regions(1920, 1080, 0).is_empty());
    }
}
```

Add `pub mod geometry;` to `core/src/lib.rs`.

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo nextest run -p cephalon_rust_core geometry
```

Expected: compile error — `reward_card_regions` / `CardRegion` not found.

- [ ] **Step 3: Implement**

Prepend to `core/src/geometry.rs` (the constants 243/460/24 are pixel measurements at the 1920×1080 reference resolution, copied verbatim from the parser):

```rust
/// Screen-space region of one reward card's name text, in the same pixel space
/// as the value passed for `screen_width`/`screen_height`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardRegion {
    /// left edge of the card
    pub x: u32,
    /// card width
    pub width: u32,
    /// bottom edge of the item-name text block
    pub text_bottom: u32,
    /// height of one line of item-name text
    pub line_height: u32,
}

/// Positions of the reward cards on the relic reward screen, scaled from the
/// 1920x1080 reference layout. `count` is the squad size (1-4); anything else
/// yields no regions. Index i is the parser's OCR slot i.
pub fn reward_card_regions(screen_width: u32, screen_height: u32, count: usize) -> Vec<CardRegion> {
    let middle = screen_width / 2;
    let frame_width = (screen_width * 243) / 1920;
    let frame_bottom = (screen_height * 460) / 1080;
    let text_height = (screen_height * 24) / 1080;
    let start_points = match count {
        4 => vec![
            middle - frame_width * 2,
            middle - frame_width,
            middle,
            middle + frame_width,
        ],
        3 => vec![
            middle - ((3 * frame_width) / 2),
            middle + (frame_width / 2),
            middle - (frame_width / 2),
        ],
        2 => vec![middle - frame_width, middle],
        1 => vec![middle - (frame_width / 2)],
        _ => Vec::new(),
    };
    start_points
        .into_iter()
        .map(|x| CardRegion {
            x,
            width: frame_width,
            text_bottom: frame_bottom,
            line_height: text_height,
        })
        .collect()
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo nextest run -p cephalon_rust_core geometry
```

Expected: 4 tests PASS.

- [ ] **Step 5: Refactor the parser to use it**

In `core/src/relic_screen_parser.rs`, replace the inline math (lines ~54-75: `width`/`height`/`middle`/`frame_width`/`frame_bottom`/`text_height`/`start_points`) with:

```rust
use crate::geometry::reward_card_regions;
```

```rust
    let regions = reward_card_regions(img.width(), img.height(), amount.len());
```

and adapt the crop loop: the iteration becomes over `regions` instead of `start_points`, where the previous `p` is `region.x`, `frame_width` is `region.width`, `frame_bottom` is `region.text_bottom`, and `text_height` is `region.line_height`. Keep every crop expression numerically identical, e.g. the naive crop becomes:

```rust
let new = img.crop(
    region.x,
    region.text_bottom - (region.line_height * i),
    region.width,
    region.line_height * i,
);
```

- [ ] **Step 6: Run the full OCR test suite to prove the refactor changed nothing**

```bash
cargo nextest run -p cephalon_rust_core
```

Expected: all tests pass, including the 6 `relic_screen_parser::tests` screenshot tests.

- [ ] **Step 7: Commit**

```bash
git add core/src/geometry.rs core/src/relic_screen_parser.rs core/src/lib.rs
git commit -m "extract reward card geometry into shared module"
```

---

### Task 4: Replace `State` with the `Event` enum

Frontends need lifecycle, not just data: opened (show window, render placeholders), resolved (fill in prices), closed (hide window). Also fixes a real modeling bug: today forma and not-yet-resolved are both `None` — indistinguishable. Make invalid states unrepresentable.

**Files:**
- Create: `core/src/event.rs`
- Delete: `core/src/state.rs`
- Modify: `core/src/lib.rs` (module decl, sender type, send sites)
- Modify: `core/src/items/items.rs` (add `PartialEq` derive to `Item`)
- Modify: `cli/src/main.rs` (consume events)

**Interfaces:**
- Consumes: nothing new
- Produces (used by Tasks 5 and 7):

```rust
// cephalon_rust_core::event
pub enum RewardSlot { Pending, Forma, Item { item: Item, price: Option<u32> } }
pub enum Event {
    RewardScreenOpened { count: usize },
    RewardsResolved(Vec<RewardSlot>),
    RewardScreenClosed,
}
```

`Engine::run` signature becomes `pub async fn run(self, sender: Sender<Event>) -> Result<(), EngineRunError>` (same error type for now; Task 6 revisits).

- [ ] **Step 1: Create `core/src/event.rs`**

```rust
use crate::items::items::Item;

/// One reward card slot, indexed to match `geometry::reward_card_regions`.
#[derive(Debug, Clone, PartialEq)]
pub enum RewardSlot {
    /// OCR hasn't identified this card yet
    Pending,
    /// forma blueprint — has no market price
    Forma,
    /// identified item; price is None if the market lookup failed
    Item { item: Item, price: Option<u32> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    RewardScreenOpened { count: usize },
    RewardsResolved(Vec<RewardSlot>),
    RewardScreenClosed,
}
```

Add `PartialEq` to `Item`'s derives in `core/src/items/items.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
```

In `core/src/lib.rs`: replace `pub mod state;` with `pub mod event;`, delete `core/src/state.rs`, change `use state::State;` to `use event::{Event, RewardSlot};`.

- [ ] **Step 2: Rewrite the send sites in `core/src/lib.rs`**

In `Engine::run`, the channel is now `Sender<Event>`. Inside the relic-screen task (currently `core/src/lib.rs:71-129`):

At activation (right after `while let Some(amount) = rx.recv().await {`):

```rust
let _ = sender.send(Event::RewardScreenOpened { count: amount }).await;
```

Replace the `sender.send(State { relic_rewards: ... })` block with slot mapping:

```rust
let slots = total_results
    .iter()
    .map(|x| async move {
        match x {
            None => RewardSlot::Pending,
            Some(ItemOrForma::Forma1X) | Some(ItemOrForma::Forma2X) => RewardSlot::Forma,
            Some(ItemOrForma::Item(item)) => RewardSlot::Item {
                item: item.clone(),
                price: item.price().await.ok(),
            },
        }
    })
    .collect::<FuturesOrdered<_>>()
    .collect::<Vec<_>>()
    .await;
let _ = sender.send(Event::RewardsResolved(slots)).await;
```

(`FuturesOrdered` — order must match card slots; the old code's `FuturesUnordered` here was a latent ordering bug, fix it in passing. Import from `futures::stream::FuturesOrdered`.)

After the attempt loop ends (both the early-`break` path and exhausting 10 attempts):

```rust
let _ = sender.send(Event::RewardScreenClosed).await;
```

- [ ] **Step 3: Update `cli/src/main.rs` to consume events**

```rust
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(100);

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                Event::RewardScreenOpened { count } => println!("reward screen opened ({count} cards)"),
                Event::RewardsResolved(slots) => {
                    let summary = slots
                        .iter()
                        .map(|s| match s {
                            RewardSlot::Pending => "…".to_string(),
                            RewardSlot::Forma => "forma".to_string(),
                            RewardSlot::Item { item, price: Some(p) } => format!("{} {p}p", item.name),
                            RewardSlot::Item { item, price: None } => format!("{} ?p", item.name),
                        })
                        .collect::<Vec<_>>()
                        .join(" | ");
                    println!("{summary}");
                }
                Event::RewardScreenClosed => println!("reward screen closed"),
            }
        }
    });
```

with `use cephalon_rust_core::{event::{Event, RewardSlot}, Engine};`.

- [ ] **Step 4: Check and test**

```bash
cargo check --workspace
cargo nextest run
```

Expected: clean check, all tests pass (no test currently covers the event flow — that lands in Task 5).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "replace State with Event enum, model forma/pending distinctly"
```

---

### Task 5: Extract the reward session + integration test

Pull the capture→OCR→price loop out of `Engine::run`'s spawned task into a testable `run_reward_session` function behind a `CaptureSource` trait. Then integration-test the full event sequence (opened → resolved → closed) against a real screenshot fixture — no game needed.

**Files:**
- Create: `core/src/reward_session.rs`
- Modify: `core/src/lib.rs` (use the new module; slim the spawned task)
- Test: `core/tests/reward_session.rs`

**Interfaces:**
- Consumes: `Event`/`RewardSlot` (Task 4), `parse_relic_screen` (existing)
- Produces (used by Tasks 6, 7):

```rust
// cephalon_rust_core::geometry (addition)
/// screen-space rect of the game window, global/virtual-desktop coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRect { pub x: i32, pub y: i32, pub width: u32, pub height: u32 }

// cephalon_rust_core::event (amendment to Task 4's enum)
// RewardScreenOpened gains the game window's screen rect so frontends can
// position UI relative to the game window, NOT the monitor — Warframe may be
// borderless on half an ultrawide or on a secondary monitor. None = unknown
// (e.g. future monitor-capture fallback); frontends then assume monitor-sized.
RewardScreenOpened { count: usize, window: Option<WindowRect> },

// cephalon_rust_core::reward_session
pub trait CaptureSource: Send + Sync + 'static {
    fn capture(&self) -> anyhow::Result<image::DynamicImage>;
}
pub async fn run_reward_session(
    capture: &dyn CaptureSource,
    items: &HashMap<String, Item>,
    sender: &Sender<Event>,
    count: usize,
    window_rect: Option<WindowRect>,
    session_duration: Duration,
)
pub const REWARD_PICK_WINDOW: Duration = Duration::from_secs(15);
```

Amendment note (user requirement, added after Task 4 dispatched): add the `WindowRect` struct to `core/src/geometry.rs`, add the `window` field to `Event::RewardScreenOpened`, update the cli match arm to print it, and have `run_reward_session` forward it verbatim into the `RewardScreenOpened` it sends. The integration test passes `Some(WindowRect { x: 0, y: 0, width: img.width(), height: img.height() })` and asserts it round-trips in the opened event.

- [ ] **Step 1: Write the failing integration test**

Create `core/tests/reward_session.rs`:

```rust
use std::{collections::HashMap, env, path::Path, time::Duration};

use cephalon_rust_core::{
    event::{Event, RewardSlot},
    items::{cached_get_item_identifiers, cached_items_and_sets, items::Item},
    reward_session::{run_reward_session, CaptureSource},
};
use image::{DynamicImage, ImageReader};

struct StaticCapture(DynamicImage);

impl CaptureSource for StaticCapture {
    fn capture(&self) -> anyhow::Result<DynamicImage> {
        Ok(self.0.clone())
    }
}

async fn items() -> HashMap<String, Item> {
    let cache_path = env::var("CACHE_PATH").unwrap();
    let cache_path = Path::new(&cache_path);
    let identifiers = cached_get_item_identifiers(cache_path).await.unwrap();
    let (items, _) = cached_items_and_sets(cache_path, &identifiers).await.unwrap();
    items
}

#[tokio::test]
async fn full_session_event_sequence() {
    let img = ImageReader::open("test_rewards_screens/1.png")
        .unwrap()
        .decode()
        .unwrap();
    let capture = StaticCapture(img);
    let items = items().await;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(100);

    run_reward_session(&capture, &items, &tx, 4, Duration::from_secs(2)).await;
    drop(tx);

    let mut events = Vec::new();
    while let Some(e) = rx.recv().await {
        events.push(e);
    }

    assert_eq!(events.first(), Some(&Event::RewardScreenOpened { count: 4 }));
    assert_eq!(events.last(), Some(&Event::RewardScreenClosed));

    let resolved = events
        .iter()
        .filter_map(|e| match e {
            Event::RewardsResolved(slots) => Some(slots),
            _ => None,
        })
        .last()
        .expect("at least one RewardsResolved event");

    let names = resolved
        .iter()
        .map(|s| match s {
            RewardSlot::Pending => "PENDING".to_string(),
            RewardSlot::Forma => "FORMA".to_string(),
            RewardSlot::Item { item, .. } => item.name.clone(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "FORMA".to_string(),
            "Okina Prime Handle".to_string(),
            "Baruuk Prime Chassis Blueprint".to_string(),
            "Shade Prime Systems".to_string(),
        ]
    );
}
```

(Expected slot contents match the existing `_1` parser test. Prices aren't asserted — they're live market data; only that items resolve.)

- [ ] **Step 2: Run it to verify it fails**

```bash
cargo nextest run -p cephalon_rust_core --test reward_session
```

Expected: compile error — `reward_session` module doesn't exist.

- [ ] **Step 3: Implement `core/src/reward_session.rs`**

Move the loop body from `core/src/lib.rs:74-127` into:

```rust
use std::{collections::HashMap, time::Duration};

use futures::stream::{FuturesOrdered, StreamExt};
use image::DynamicImage;
use tokio::{
    sync::mpsc::Sender,
    time::{sleep, Instant},
};
use tracing::*;

use crate::{
    debug_write_image,
    event::{Event, RewardSlot},
    items::items::Item,
    relic_screen_parser::{parse_relic_screen, ItemOrForma},
};

/// how long the in-game reward pick window stays on screen
pub const REWARD_PICK_WINDOW: Duration = Duration::from_secs(15);

const MAX_ATTEMPTS: usize = 10;

pub trait CaptureSource: Send + Sync + 'static {
    fn capture(&self) -> anyhow::Result<DynamicImage>;
}

pub async fn run_reward_session(
    capture: &dyn CaptureSource,
    items: &HashMap<String, Item>,
    sender: &Sender<Event>,
    count: usize,
    session_duration: Duration,
) {
    let started = Instant::now();
    let _ = sender.send(Event::RewardScreenOpened { count }).await;

    let mut total_results: Vec<Option<ItemOrForma>> = (0..count).map(|_| None).collect();
    for attempt in 0..MAX_ATTEMPTS {
        event!(Level::INFO, "relic screen run {attempt}");
        sleep(Duration::from_millis(1000)).await;
        let image = match capture.capture() {
            Ok(img) => img,
            Err(e) => {
                event!(Level::WARN, "capture failed mid-session: {e}");
                break;
            }
        };
        debug_write_image(&image, &format!("reward_capture_{attempt}"));
        let results = parse_relic_screen(
            &image,
            &total_results
                .iter()
                .enumerate()
                .filter(|(_, x)| x.is_none())
                .map(|(i, _)| i)
                .collect(),
            items,
        )
        .await;
        total_results = total_results
            .into_iter()
            .zip(results)
            .map(|(a, b)| a.or(b))
            .collect();
        let finished = total_results.iter().all(|x| x.is_some());

        let slots = total_results
            .iter()
            .map(|x| async move {
                match x {
                    None => RewardSlot::Pending,
                    Some(ItemOrForma::Forma1X) | Some(ItemOrForma::Forma2X) => RewardSlot::Forma,
                    Some(ItemOrForma::Item(item)) => RewardSlot::Item {
                        item: item.clone(),
                        price: item.price().await.ok(),
                    },
                }
            })
            .collect::<FuturesOrdered<_>>()
            .collect::<Vec<_>>()
            .await;
        let _ = sender.send(Event::RewardsResolved(slots)).await;

        if finished {
            event!(Level::INFO, "relic screen run found all, finishing early");
            break;
        }
    }

    // keep the overlay up for the whole pick window even if OCR finished early
    if let Some(rest) = session_duration.checked_sub(started.elapsed()) {
        sleep(rest).await;
    }
    let _ = sender.send(Event::RewardScreenClosed).await;
}
```

Notes:
- `(a, b) -> a.or(b)` replaces the old 3-arm match — identical semantics.
- The old `debug_write_image` is `pub(crate)`; it stays usable since this module is in-crate.
- In `core/src/lib.rs`: add `pub mod reward_session;`, and shrink the spawned task to call `run_reward_session(&capture, &items, &sender, amount, REWARD_PICK_WINDOW).await` with an xcap-backed capture:

```rust
pub struct WindowCapture(xcap::Window);

impl CaptureSource for WindowCapture {
    fn capture(&self) -> anyhow::Result<DynamicImage> {
        Ok(DynamicImage::ImageRgba8(self.0.capture_image()?))
    }
}
```

(Adapt to the post-Task-1 xcap API. If `xcap::Window` isn't `Sync`, wrap in a `Mutex` or capture per call — check the upgraded API. Task 6 restructures who owns this anyway; keep it compiling.)

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo nextest run -p cephalon_rust_core --test reward_session
```

Expected: PASS in roughly 5-15s (OCR attempts + live price fetches; needs network + warmed `CACHE_PATH`).

- [ ] **Step 5: Run everything**

```bash
cargo nextest run
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "extract reward session behind CaptureSource, add event sequence test"
```

---

### Task 6: Start-and-forget engine

Remove the "Warframe must already be running" requirement: the log watcher waits for EE.log to exist, and the Warframe window is looked up when a reward screen actually triggers (not at startup). `Engine::run` becomes an endless loop that never errors out in normal operation.

**Files:**
- Modify: `core/src/log_watcher/mod.rs` (wait for the file)
- Modify: `core/src/lib.rs` (window lookup per session; drop `EngineRunError::WarframeNotRunning`; drop the startup screenshot)
- Modify: `cli/src/main.rs` (signature change fallout)

**Interfaces:**
- Consumes: `run_reward_session` (Task 5)
- Produces: `pub async fn run(self, sender: Sender<Event>)` — no `Result`, runs forever. `EngineRunError` deleted. Task 7 relies on run-forever semantics.

- [ ] **Step 1: Make the log watcher wait for EE.log**

In `core/src/log_watcher/mod.rs`, `watcher()` currently does `File::open(...).unwrap()` before spawning. Move file opening inside the spawned task with a wait loop:

```rust
pub async fn watcher() -> tokio::sync::mpsc::Receiver<LogEntry> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    tokio::spawn(async move {
        let path = get_default_path();
        let mut file = loop {
            match File::open(&path).await {
                Ok(f) => break BufReader::new(f),
                Err(_) => {
                    event!(Level::INFO, "EE.log not found yet, waiting");
                    sleep(Duration::from_secs(5)).await;
                }
            }
        };
        file.seek(SeekFrom::End(0)).await.unwrap();
        let mut buffer = Vec::with_capacity(50);
        loop {
            // ... existing read loop unchanged ...
        }
    });
    rx
}
```

(add `use tracing::*;` — the module doesn't import it yet despite the crate using tracing elsewhere.)

- [ ] **Step 2: Restructure `Engine::run`**

In `core/src/lib.rs`:
- Delete `EngineRunError` and the startup block that finds the window and takes the `initial` screenshot (`core/src/lib.rs:53-62`).
- Store items shared: `pub struct Engine { items: Arc<HashMap<String, Item>> }` (`Arc` because each session task needs them).
- New run shape:

```rust
pub async fn run(self, sender: Sender<Event>) {
    let mut squad_size = 4;
    let mut receiver = watcher().await;

    while let Some(entry) = receiver.recv().await {
        match entry {
            LogEntry::ScriptInfo { script, content }
                if script == "ProjectionRewardChoice" && content == "Relic rewards initialized" =>
            {
                event!(Level::INFO, "relic reward screen detected");
                match find_warframe_window() {
                    Some(window) => {
                        // window rect in screen coords so frontends can position UI
                        // relative to the game window (half-ultrawide, second monitor)
                        let rect = window_rect(&window);
                        let capture = WindowCapture(window);
                        let items = self.items.clone();
                        let sender = sender.clone();
                        let count = squad_size;
                        tokio::spawn(async move {
                            run_reward_session(&capture, &items, &sender, count, rect, REWARD_PICK_WINDOW)
                                .await;
                        });
                    }
                    None => event!(Level::WARN, "reward screen detected but no Warframe window"),
                }
            }
            LogEntry::NetInfo(x) if x == "Num session players: 1" => squad_size = 1,
            LogEntry::NetInfo(x) if x == "Num session players: 2" => squad_size = 2,
            LogEntry::NetInfo(x) if x == "Num session players: 3" => squad_size = 3,
            LogEntry::NetInfo(x) if x == "Num session players: 4" => squad_size = 4,
            _ => {}
        }
    }
}

fn find_warframe_window() -> Option<xcap::Window> {
    xcap::Window::all()
        .ok()?
        .into_iter()
        .find(|x| x.title().unwrap_or_default() == "Warframe")
}

fn window_rect(window: &xcap::Window) -> Option<WindowRect> {
    // xcap exposes x()/y()/width()/height() on Window (X11: GetGeometry +
    // TranslateCoordinates). Adapt to the 0.9 API's exact shapes (they may
    // return Results). None if any lookup fails — frontends fall back to
    // assuming the game covers the monitor.
    Some(WindowRect {
        x: window.x().ok()?,
        y: window.y().ok()?,
        width: window.width().ok()?,
        height: window.height().ok()?,
    })
}
```

(`.title()` shape depends on the post-Task-1 xcap API — adapt so it still means "window titled exactly Warframe". The old per-run mpsc relay channel `relic_screen_enabler` disappears entirely.)

- [ ] **Step 3: Fix `cli/src/main.rs`**

`engine.run(tx).await?;` becomes `engine.run(tx).await;` and the function's `anyhow::Result` stays for `Engine::new`.

- [ ] **Step 4: Check and test**

```bash
cargo check --workspace
cargo nextest run
```

Expected: clean; all tests green.

- [ ] **Step 5: Smoke test the waiting behavior**

```bash
timeout 12 cargo run -p cephalon_rust_cli; cat cephalon.log | tail -5
```

Without Warframe running, expected: process idles for 12s then gets killed by timeout, `cephalon.log` shows engine startup and (if EE.log is absent) the "EE.log not found yet, waiting" lines — and critically, no crash/panic.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "engine waits for warframe instead of requiring it at startup"
```

---

### Task 7: The real overlay UI

Replace the spike's `app()` with the production overlay: engine running on a background tokio runtime, events bridged into a Freya `State`, price pills positioned via `geometry`, window visibility tied to the reward screen lifecycle.

**Files:**
- Create: `overlay/src/config.rs`
- Modify: `overlay/src/main.rs` (keep launch config from Task 2; replace `app()`)

**Interfaces:**
- Consumes: `Engine`, `Event`, `RewardSlot` (Tasks 4-6), `geometry::reward_card_regions` (Task 3), spike launch config (Task 2)
- Produces: `cephalon_rust_overlay` binary — the MVP deliverable.

- [ ] **Step 1: Create `overlay/src/config.rs`**

Same env-based pattern as the cli (source of truth: `CACHE_PATH` from `config.env` via the devshell; `MONITOR` optional):

```rust
use std::path::PathBuf;

use tokio::sync::OnceCell;

#[derive(serde::Deserialize)]
pub struct Settings {
    pub cache_path: PathBuf,
    /// index into display_info::DisplayInfo::all(); primary display when unset
    #[serde(default)]
    pub monitor: Option<usize>,
}

pub async fn settings() -> &'static Settings {
    static ONCE: OnceCell<Settings> = OnceCell::const_new();

    (ONCE
        .get_or_init(|| async {
            config::Config::builder()
                .add_source(config::Environment::default())
                .build()
                .unwrap()
                .try_deserialize()
                .unwrap()
        })
        .await) as _
}
```

- [ ] **Step 2: Rewrite `overlay/src/main.rs`**

Keep the Task 2 `main()`/launch-config code with three changes: monitor selection honors `Settings::monitor`, the window starts hidden, and `app` becomes a closure carrying the display's logical size. Full file:

```rust
mod config;

use std::{fs::OpenOptions, time::Duration};

use cephalon_rust_core::{
    event::{Event, RewardSlot},
    geometry::reward_card_regions,
    Engine,
};
use config::settings;
use freya::prelude::*;
use tracing_subscriber::{fmt, prelude::*, Registry};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::WindowLevel,
};

fn pick_display(monitor: Option<usize>) -> display_info::DisplayInfo {
    let displays = display_info::DisplayInfo::all().unwrap_or_default();
    monitor
        .and_then(|i| displays.get(i).cloned())
        .or_else(|| displays.iter().find(|d| d.is_primary).cloned())
        .or_else(|| displays.first().cloned())
        .expect("no displays found")
}

fn main() {
    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("cephalon.log")
        .unwrap();
    let subscriber = Registry::default().with(fmt::layer().with_writer(log_file));
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // settings() is async; resolve it on a throwaway runtime before the UI starts
    let monitor = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { settings().await.monitor });

    let display = pick_display(monitor);
    // +1/-1 px fudge: transparent windows at exact monitor size go black (Orbolay's workaround)
    let size = PhysicalSize::new(
        (display.width + 1) as f64 * display.scale_factor as f64,
        (display.height - 1) as f64 * display.scale_factor as f64,
    );
    let position = PhysicalPosition::new(display.x, display.y);
    // freya lays out in logical points; geometry needs the same space
    let logical = (display.width + 1, display.height - 1);

    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(move || app(logical.0, logical.1, (display.x, display.y)))
                .with_title("cephalon")
                .with_decorations(false)
                .with_transparency(true)
                .with_background(Color::TRANSPARENT)
                .with_window_attributes(move |mut w, _event_loop| {
                    w = w
                        .with_inner_size(size)
                        .with_resizable(false)
                        .with_visible(false)
                        .with_window_level(WindowLevel::AlwaysOnTop)
                        .with_position(position);

                    #[cfg(target_os = "linux")]
                    {
                        use winit::platform::wayland::WindowAttributesExtWayland;
                        w = WindowAttributesExtWayland::with_name(w, "cephalon", "cephalon");
                    }

                    w
                }),
        ),
    );
}

fn app(width: u32, height: u32, display_origin: (i32, i32)) -> impl IntoElement {
    let mut screen = use_state(|| Option::<RewardScreen>::None);

    use_hook(move || {
        Platform::get().with_window(None, |w| {
            let _ = w.set_cursor_hittest(false);
        });

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Event>(100);

        // engine lives on its own tokio runtime; the UI runtime is not tokio
        std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async move {
                let settings = settings().await;
                let engine = Engine::new(settings.cache_path.clone())
                    .await
                    .expect("engine init failed");
                engine.run(tx).await;
            });
        });

        // tokio mpsc recv is runtime-agnostic, safe to await on freya's executor
        spawn_forever(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    Event::RewardScreenOpened { count, window } => {
                        screen.set(Some(RewardScreen {
                            slots: vec![RewardSlot::Pending; count],
                            window,
                        }));
                    }
                    Event::RewardsResolved(resolved) => {
                        // keep the window rect from Opened; update slots only
                        let window = screen.read().as_ref().and_then(|s| s.window);
                        screen.set(Some(RewardScreen { slots: resolved, window }));
                    }
                    Event::RewardScreenClosed => {
                        screen.set(None);
                    }
                }
            }
        });
    });

    use_side_effect(move || {
        let visible = screen.read().is_some();
        Platform::get().with_window(None, move |w| w.set_visible(visible));
    });

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .maybe_child(screen.read().clone().map(|s| {
            // labels live inside the GAME WINDOW's rect, not the monitor's:
            // warframe may be borderless on half an ultrawide or another monitor.
            // rect coords are global screen space; the overlay window starts at
            // display_origin, so translate. No rect -> assume game covers display.
            let game = s.window.unwrap_or(cephalon_rust_core::geometry::WindowRect {
                x: display_origin.0,
                y: display_origin.1,
                width,
                height,
            });
            RewardLabels {
                slots: s.slots,
                offset_x: game.x - display_origin.0,
                offset_y: game.y - display_origin.1,
                width: game.width,
                height: game.height,
            }
        }))
}

#[derive(PartialEq, Clone)]
struct RewardScreen {
    slots: Vec<RewardSlot>,
    window: Option<cephalon_rust_core::geometry::WindowRect>,
}

#[derive(PartialEq)]
struct RewardLabels {
    slots: Vec<RewardSlot>,
    /// game window origin relative to the overlay window
    offset_x: i32,
    offset_y: i32,
    /// game window size — geometry is computed in the game's pixel space
    width: u32,
    height: u32,
}

impl Component for RewardLabels {
    fn render(&self) -> impl IntoElement {
        let regions = reward_card_regions(self.width, self.height, self.slots.len());
        self.slots.iter().zip(regions).fold(
            rect()
                .position(
                    Position::new_absolute()
                        .top(self.offset_y as f32)
                        .left(self.offset_x as f32),
                )
                .width(Size::fill())
                .height(Size::fill()),
            |el, (slot, region)| {
                let text = match slot {
                    RewardSlot::Pending => "…".to_string(),
                    RewardSlot::Forma => "—".to_string(),
                    RewardSlot::Item { price: Some(p), .. } => format!("{p}p"),
                    RewardSlot::Item { price: None, .. } => "?".to_string(),
                };
                el.child(
                    rect()
                        .position(
                            Position::new_absolute()
                                .top((region.text_bottom + region.line_height) as f32)
                                .left(region.x as f32),
                        )
                        .width(Size::px(region.width as f32))
                        .direction(Direction::Horizontal)
                        .main_align(Alignment::Center)
                        .child(
                            rect()
                                .padding(Gaps::new(4., 14., 4., 14.))
                                .corner_radius(CornerRadius::new_all(12.))
                                .background(Color::new(0xCC14141A))
                                .child(
                                    label()
                                        .font_size(26.)
                                        .font_weight(FontWeight::BOLD)
                                        .color(Color::WHITE)
                                        .text(text),
                                ),
                        ),
                )
            },
        )
    }
}
```

Notes for the implementer:
- **Game-window-relative positioning is a hard requirement:** Warframe borderless on the left half of an ultrawide, or on a secondary monitor, must still get correctly-placed labels. That's why regions are computed from the game rect, not the display. If the game rect's center lies outside the overlay's display, log a warning (a follow-the-game window move is a recorded follow-up, not MVP — the `MONITOR` env override covers it).
- `Color::new(0xCC14141A)` is ARGB: ~80% opaque near-black pill.
- The label row sits one text-line below the card's name text (`text_bottom + line_height`), i.e. just under the card content — matches the approved mockup.
- If the display scale factor is not 1.0, verify label alignment in Task 8; if labels land offset, the logical size passed to `app()` needs dividing by `scale_factor` (COSMIC at scale 1 won't show the difference).
- `futures-timer` is no longer used after the spike is replaced — remove it from `overlay/Cargo.toml`.

- [ ] **Step 3: Build**

```bash
cargo check -p cephalon_rust_overlay
cargo build -p cephalon_rust_overlay
```

Expected: clean. If Freya rc.24 builder-API names differ slightly from this plan (pre-release!), check the Orbolay source (github.com/SpikeHD/Orbolay, `src/main.rs`, `crates/orbolay-ui`) for ground truth — it ships on exactly this version.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "overlay: reward price pills wired to engine events"
```

---

### Task 8: End-to-end verification + docs

Verify the pipeline without playing a mission (synthetic EE.log lines), then document how to run it. A real fissure run is the final acceptance test but needs the user in-game.

**Files:**
- Modify: `README.md`
- No code changes expected (fixes only if verification finds bugs)

**Interfaces:**
- Consumes: everything
- Produces: verified MVP + updated README.

- [ ] **Step 1: Synthetic end-to-end test (no mission needed)**

With Warframe running (sitting in the orbiter/menu is fine, window titled "Warframe", borderless) and the overlay running (`cargo run -p cephalon_rust_overlay`), inject a fake log line:

```bash
echo '9999.999 Script [Info]: ProjectionRewardChoice.lua: Relic rewards initialized' >> ~/.local/share/Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log
```

Expected sequence:
1. Overlay window appears above the game within ~1s.
2. Four "…" pills render at the card positions (squad size defaults to 4).
3. OCR finds no items on a non-reward screen, so pills stay "…" through 10 attempts (~10s).
4. After ~15s total, the overlay hides again.
5. `cephalon.log` shows the session events; no panics.

If Warframe's EE.log truncation interferes, check the path matches `log_watcher::get_default_path()`.

- [ ] **Step 2: Fix what step 1 surfaces**

Most likely issues and where they live: labels misaligned (scale factor handling, Task 7 note), window not hiding (`use_side_effect`/`set_visible`), window behind game (re-check Task 2 spike findings — escalate if regressed). Commit fixes individually with descriptive messages.

- [ ] **Step 2b: Non-fullscreen window placement check**

Repeat step 1 with Warframe in a windowed/borderless configuration that does NOT cover the monitor (e.g. resized to roughly the left half of the screen). Expected: the "…" pills appear inside the Warframe window at card positions scaled to the window's size — not stretched across the whole monitor. This verifies the `WindowRect` plumbing end to end (xcap geometry under XWayland is the empirical unknown here — if coordinates come back wrong/zeroed under COSMIC's XWM, record what xcap returned and escalate).

- [ ] **Step 3: Real fissure run (user acceptance)**

Ask the user to run a relic fissure with the overlay up. Acceptance: prices appear over the cards while the pick timer runs, match warframe.market sanity, and the overlay disappears after picking. Debug material lands in `debug_img_out/` (debug builds) and `cephalon.log`.

- [ ] **Step 4: Update README**

Replace `README.md` content:

```markdown
# cephalon rust

ordis' more helpful brother — watches your warframe session and overlays relic
reward prices (warframe.market) over the reward cards, in game.

## crates

- `core` — frontend-agnostic engine: EE.log watcher, screen capture + OCR,
  warframe.market prices. emits an event stream any frontend can consume.
- `overlay` — freya/winit transparent click-through overlay (wayland; borderless warframe)
- `cli` — headless frontend, prints events

## running

inside the dev shell (`nix develop`):

```sh
cargo run -p cephalon_rust_overlay
```

start it whenever — it waits for warframe. `CACHE_PATH` comes from `config.env`;
optional `MONITOR=<index>` picks a non-primary display.

# TODO

- generalize the determined price from all orders algorithm so the user of the
  library can specify their own logic
- determine players in group with OCR
- exclusive fullscreen support, x11 host support, windows support (in that order)
- package the overlay in the flake (skia-safe downloads binaries at build time,
  needs vendoring for the sandbox)
- find a reliable EE.log line for "reward picked" instead of the 15s timeout
- try https://crates.io/crates/crabgrab and cleanup flake inputs
```

- [ ] **Step 5: Final check + commit**

```bash
cargo nextest run
git add -A
git commit -m "readme + e2e verification notes"
```
