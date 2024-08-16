use gtk4::{prelude::*, Application, ApplicationWindow, Label};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use items::db_reset;
use xcap::Window;

pub mod config;
pub mod items;

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
    let app = Application::builder().application_id("org.github.cramt.cephalon_rust").build();
    app.connect_activate(|x|{
        let window = ApplicationWindow::builder().application(x).child(&Label::new(Some("test"))).build();
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Bottom, true);
        window.present();

    });
}
