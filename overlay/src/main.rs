mod config;

use std::fs::OpenOptions;

use cephalon_rust_core::{
    event::{Event, RewardSlot},
    geometry::reward_card_regions,
    Engine,
};
use config::settings;
use freya::prelude::*;
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};
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
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("warn,cephalon_rust_core=info,cephalon_rust_overlay=info")
    });
    let subscriber = Registry::default()
        .with(filter)
        .with(fmt::layer().with_writer(log_file));
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
                        // keep the window rect from Opened; update slots only.
                        // ignore Resolved with no screen open — resurrecting one
                        // here would lose the window rect and flash an orphan overlay
                        if let Some(window) = screen.read().as_ref().map(|s| s.window) {
                            screen.set(Some(RewardScreen { slots: resolved, window }));
                        }
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
