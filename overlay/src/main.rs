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
