#![allow(warnings)]
mod app;
mod components;
mod pages;
mod services;
mod models;
mod utils;
mod state;

use app::AppRoot;

fn main() {
    console_error_panic_hook::set_once();
    yew::Renderer::<AppRoot>::new().render();
}
