build_overlay:
    CARGO_BUILD_TARGET=x86_64-pc-windows-gnu cargo build -p cephalon_rust_overlay

run_overlay:
    just build_overlay && WINEPREFIX=/home/cramt/.local/share/Steam/steamapps/compatdata/230410/pfx/ "/home/cramt/.local/share/Steam/steamapps/common/Proton 9.0 (Beta)/files/bin/wine64" ./target/x86_64-pc-windows-gnu/debug/cephalon_rust_overlay.exe

run_daemon:
    just build_overlay
    cargo run -p cephalon_rust_daemon
