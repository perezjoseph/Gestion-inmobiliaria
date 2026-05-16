mod app;
mod components;
mod hooks;
mod pages;
mod services;
mod types;
mod utils;

fn main() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"[INIT] pre-renderer".into());
    yew::Renderer::<app::App>::new().render();

    if let Some(window) = web_sys::window()
        && let Some(document) = window.document()
        && let Some(el) = document.get_element_by_id("loading")
    {
        el.remove();
    }
}
