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

start it whenever — it waits for warframe. first launch downloads item data
from warframe.market into `CACHE_PATH` (from `config.env`), which takes a few
minutes; after that it's instant. optional `MONITOR=<index>` picks a
non-primary display for the overlay. `RUST_LOG` overrides the log filter.

labels are positioned relative to the warframe window, so borderless on half
an ultrawide or a secondary monitor works too.

# TODO

- generalize the determined price from all orders algorithm so the user of the
  library can specify their own logic (v2 note: order "region" is gone, we
  filter by user locale — maybe just drop that filter)
- determine players in group with OCR
- exclusive fullscreen support, x11 host support, windows support (in that order)
- package the overlay in the flake (skia-safe downloads binaries at build time,
  needs vendoring for the sandbox)
- find a reliable EE.log line for "reward picked" instead of the 15s timeout
- progress feedback during the first-run item-data fetch (it's minutes of silence)
- consider vulkan-loader in the flake (freya warns and falls back to GL, which works)
- cleanup flake inputs
