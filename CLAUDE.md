# cephalon_rust

warframe companion: EE.log watcher → screenshot + OCR of relic reward screens →
warframe.market prices → in-game overlay. See README.md for the crate layout.

## Architecture rule

`core` is frontend-agnostic and must NEVER depend on winit/freya/display-info or
anything display-server-related. Frontends (overlay, cli, a future windows one)
consume `Engine::run`'s `Event` stream and the pure-math `geometry` module.

## Building & testing

- **Every cargo command must run inside the dev shell**: `nix develop -c <cmd>`.
  Global cargo has the wrong toolchain and lacks the `DETECTION_MODEL` /
  `RECOGNITION_MODEL` build-time env vars (OCR models embedded via
  `include_bytes!`, supplied by flake inputs).
- The devshell exports `config.env` (`CACHE_PATH=./app_cache`).
- `cargo nextest run` — tests hit the LIVE warframe.market v2 API on first run
  and cache into `CACHE_PATH` (per test CWD, so `core/app_cache` for core
  tests). Warm runs are fast; cold OCR-test runs take minutes.
- Deps are optimized even in dev (`[profile.dev.package."*"] opt-level = 3`),
  so debug-mode tests are fine.
- warframe.market **v1 is dead** — everything uses v2 (`data` envelope,
  `_v2`-suffixed cache files). Don't reintroduce v1 endpoints.

## Overlay specifics

- Freya is pinned `=0.4.0-rc.24` — it's the post-Dioxus rewrite: builder API
  (`rect().width(Size::fill())`), struct components with `impl Component`,
  `use_state`/`State<T>`, `spawn_forever`. NO `rsx!`, no 0.3 idioms.
  Ground truth for API questions: github.com/SpikeHD/Orbolay (same version).
- Freya's executor is NOT tokio: in UI-side async, only runtime-agnostic awaits
  (tokio mpsc recv is ok; `tokio::time::sleep` panics). The engine runs on its
  own thread with its own tokio runtime.
- Careful with `State<T>`: holding a `read()`/`peek()` guard across a `set()`
  panics at runtime (edition-2021 `if let` scrutinee temporaries!). Hoist reads
  into a `let` before writing.
- Manual overlay smoke test without playing a mission: run the overlay, then
  append to EE.log:
  `echo '9999.999 Script [Info]: ProjectionRewardChoice.lua: Relic rewards initialized' >> ~/.local/share/Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log`

## Conventions

- Commit messages: lowercase, casual, one line (see `git log --oneline`). No
  Co-Authored-By, no "Generated with".
- Comments explain the why, never the what.
- Make invalid states unrepresentable (see `RewardSlot`).
- Design docs in `docs/superpowers/specs/`, plans in `docs/superpowers/plans/`.
