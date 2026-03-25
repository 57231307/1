mod app;
mod components;
mod pages;
mod services;
mod models;
mod utils;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}
