mod app;
mod components;
mod pages;
mod services;
mod types;
mod utils;

fn main() {
    yew::Renderer::<app::App>::new().render();
}
