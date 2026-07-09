# Relic Reward Overlay — Design

**Date:** 2026-07-09
**Status:** Approved
**Goal:** Revitalize cephalon_rust by giving the existing relic-reward pipeline an in-game overlay UI, built on the same tech as [Orbolay](https://github.com/SpikeHD/Orbolay) (Freya + winit, Skia rendering, transparent click-through window).

## Problem

The core pipeline already works: the EE.log watcher detects the relic reward screen, `xcap` screenshots the Warframe window, `ocrs` OCRs the reward names, and warframe.market supplies platinum prices. But the only frontend is a CLI that prints a HashMap — useless mid-mission. The user should see prices in-game, over the reward cards, without alt-tabbing.

## Scope

**MVP (this spec):**
- Overlay showing platinum prices as labels aligned under each reward card.
- Target environment: Wayland host (COSMIC), Warframe via Proton running borderless, under XWayland or native Wayland (`PROTON_ENABLE_WAYLAND`).

**Explicitly deferred:** exclusive-fullscreen support, X11 host support, Windows support, squad-member OCR, timers, any interactive UI.

## Architectural constraint: frontend-agnostic core

`core` must never depend on winit, Freya, or anything display-server-related. Its public contract for frontends is:

- `Engine::new(cache_path)` + `Engine::run(sender)` emitting `Event`s over a tokio mpsc channel.
- A pure-math `geometry` module for positioning UI relative to screen dimensions.

Any frontend on any platform (this overlay, the CLI, a future Windows build) consumes the same event stream. `xcap` and the log watcher are already cross-platform and stay in core.

## Components

### `core` changes

1. **`geometry` module (new):** extract the reward-card position math from `relic_screen_parser` (`start_points`, `frame_width`, `frame_bottom`, `text_height` — all scaled from a 1920×1080 reference) into a public function, roughly `reward_card_regions(width, height, count) -> Vec<Rect>`. The parser crops with it; frontends position labels with it. One source of truth.

2. **`Event` enum replaces `State`:**

   ```rust
   pub enum Event {
       RewardScreenOpened { count: usize },
       RewardsResolved(Vec<Option<(Item, u32)>>), // streamed as OCR results land
       RewardScreenClosed,
   }
   ```

   `RewardScreenClosed` is new: fired on a timeout (~15s) after the screen opens. If a reliable EE.log line for reward selection is found later, switch to that.

3. **Wait-for-Warframe:** `Engine::run` currently errors if the Warframe window is missing. Change to a poll-and-wait loop (every few seconds); if capture fails mid-run (game closed), return to waiting. The app is start-and-forget in either launch order.

### `overlay` crate (new)

Freya `0.4.0-rc` (pinned) + winit `0.30`. Main thread runs the Freya event loop; the `Engine` runs on a background tokio runtime; events reach the UI via a channel consumed by a Freya coroutine feeding a `Signal`.

**Window recipe (Orbolay's, trimmed):**
- Undecorated, transparent background, skip taskbar, sized/positioned to the target monitor, `WindowLevel::AlwaysOnTop`, Wayland app-id `cephalon`.
- Click-through via `set_cursor_hittest(false)` at startup (empty input region on Wayland). No input handling at all — pure display. Orbolay's X11 shape hack is deferred to the X11 milestone.
- Starts hidden. `set_visible(true)` on `RewardScreenOpened`, `false` on `RewardScreenClosed`. Mapping the window fresh each reward screen is also the stacking strategy: newly mapped toplevels appear above the borderless game, and the window doesn't exist to cause problems the rest of the time.

**UI:** one root component reading `Signal<Option<RewardScreen>>`. When active, compute `reward_card_regions` for the window size and absolutely position one label per card below the card frame: `45p` for priced items, `—` for forma, `…` while unresolved. Minimal, large, high-contrast text on a subtle dark pill (readable over fissure VFX). Item names are already on the cards; don't repeat them.

**Monitor choice:** primary monitor by default; config option for monitor index.

### `cli` (existing)

Stays as the headless/debug frontend; updated to print the new `Event` stream.

## Error handling

- Engine: never exits for "Warframe not running" — waits. Capture failures mid-run return it to waiting.
- Per-card OCR or price-fetch failures already degrade to `None`; overlay shows `…`/nothing rather than erroring.
- Overlay must never steal input or crash the session; it holds no input grabs by construction.

## Testing

- **Geometry extraction:** unit tests pinning the exact pre-refactor values (pure refactor, provable).
- **Existing OCR screen tests:** must keep passing unchanged.
- **Engine event sequencing:** opened → resolved → closed-on-timeout, driven by a faked log stream.
- **Overlay UI:** `freya-testing` for label layout if cheap; otherwise manual. Real verification is the spike plus a live fissure run.

## Risks & mitigations

1. **COSMIC stacking (main risk):** does an xdg toplevel mapped mid-game render above borderless Warframe? **Step one of implementation is a throwaway spike**: transparent Freya window with a colored box, launched over running Warframe, verifying stacking + click-through. Fallback if it fails: swap the windowing layer for wlr-layer-shell (COSMIC supports it — cosmic-panel uses it), keeping all Freya UI code; interim workaround is COSMIC's manual always-on-top toggle.
2. **skia-safe under Nix:** Freya→Skia downloads prebuilt binaries at build time; fine in the dev shell, incompatible with sandboxed crane builds. MVP: overlay builds via dev shell; flake packaging of the overlay is a follow-up. `core`/`cli` stay fully flake-buildable.
3. **Freya 0.4 is pre-release:** API churn risk; pin the exact rc (Orbolay ships on rc.24).

## Milestones after MVP (recorded, not designed)

Priority order per user: exclusive-fullscreen support → X11 host support → Windows support. Plus existing README TODOs (squad OCR, price-algorithm generalization).
