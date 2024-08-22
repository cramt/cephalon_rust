run_overlay:
    cargo build -p cephalon_rust_overlay && wine64 ./target/x86_64-pc-windows-gnu/debug/cephalon_rust_overlay.exe

run_daemon:
    cargo run -p cephalon_rust_daemon
