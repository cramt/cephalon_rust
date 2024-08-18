use gtk4::{prelude::*, Application, ApplicationWindow, Fixed, Label};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use items::db_reset;
use log_watcher::watcher;
use xcap::Window;

pub mod config;
pub mod items;
pub mod log_watcher;

async fn main2() {
    let windows = Window::all().unwrap();

    println!(
        "{:?}",
        windows.iter().map(|x| x.title()).collect::<Vec<_>>()
    );
    let warframe_window = windows
        .into_iter()
        .find(|x| x.title() == "Warframe")
        .unwrap();
    let image = warframe_window.capture_image().unwrap();
    image.save("a.png").unwrap();
    db_reset().await.unwrap();
}

#[tokio::main]
async fn main(){

    watcher().await;

    gtk4::gdk::set_allowed_backends("wayland,x11,win32,macos,*");

    println!("{:?}", gtk4::gdk::DisplayManager::get().list_displays());
    let app = Application::builder().application_id("org.github.cramt.cephalon_rust").build();

    app.connect_startup(|_|{
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("style.css"));
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(|x|{
        let fixed = Fixed::builder().can_focus(false).can_target(false).focusable(false).focus_on_click(false).sensitive(false).build();
        fixed.put(&Label::builder().can_target(false).can_focus(false).focusable(false).focus_on_click(false).sensitive(false).label("test").build(), 100.0, 100.0);
        let window = ApplicationWindow::builder().application(x).child(&fixed).can_focus(false).can_target(false).focusable(false).focus_on_click(false).sensitive(false).build();
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_keyboard_mode(KeyboardMode::None);
        //window.show();

    });
    app.run();
}
