use web_sys::MouseEvent;
use yew::prelude::*;

pub fn pagination_cbs(
    page: &UseStateHandle<u64>,
    per_page: &UseStateHandle<u64>,
    reload: &UseStateHandle<u32>,
) -> (Callback<u64>, Callback<u64>) {
    let (p, r) = (page.clone(), reload.clone());
    let on_page = Callback::from(move |v: u64| {
        p.set(v);
        r.set(*r + 1);
    });
    let (pp, p2, r2) = (per_page.clone(), page.clone(), reload.clone());
    let on_pp = Callback::from(move |v: u64| {
        pp.set(v);
        p2.set(1);
        r2.set(*r2 + 1);
    });
    (on_page, on_pp)
}

pub fn delete_click_cb<T: 'static>(target: &UseStateHandle<Option<T>>) -> Callback<T> {
    let t = target.clone();
    Callback::from(move |item: T| t.set(Some(item)))
}

pub fn delete_cancel_cb<T: Clone + PartialEq + 'static>(
    target: &UseStateHandle<Option<T>>,
) -> Callback<MouseEvent> {
    let t = target.clone();
    Callback::from(move |_: MouseEvent| t.set(None))
}

pub fn new_cb<T: Clone + PartialEq + 'static>(
    reset: impl Fn() + 'static,
    state: &UseStateHandle<T>,
    value: T,
) -> Callback<MouseEvent> {
    let s = state.clone();
    Callback::from(move |_: MouseEvent| {
        reset();
        s.set(value.clone());
    })
}

pub fn cancel_cb(reset: impl Fn() + 'static) -> Callback<MouseEvent> {
    Callback::from(move |_: MouseEvent| reset())
}
