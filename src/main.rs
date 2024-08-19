use items::db_reset;
use log_watcher::watcher;
use xcap::Window;
use eframe::egui::*;

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
    //watcher().await;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 100.0])
            .with_min_inner_size([400.0, 100.0])
            .with_maximized(true)
            .with_transparent(true) 
            .with_mouse_passthrough(true)
            .with_position(egui::Pos2::new(100.0, 100.0))
            .with_resizable(false)
            .with_transparent(true)
            .with_decorations(false)
            .with_drag_and_drop(false)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native("Popups", options, Box::new(|_| Ok(Box::<MyApp>::default()))).unwrap();
}

#[derive(Default)]
struct MyApp {
    checkbox: bool,
    number: u8,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(true));
egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("hello there"));
        });
    }
}
