use std::collections::HashMap;

use web_sys::window;

/// Reads query parameters from the current URL.
/// Returns a `HashMap` of key-value pairs.
pub fn read_query_params() -> HashMap<String, String> {
    let mut params = HashMap::new();
    let Some(win) = window() else {
        return params;
    };
    let Ok(search) = win.location().search() else {
        return params;
    };
    let search = search.trim_start_matches('?');
    if search.is_empty() {
        return params;
    }
    for pair in search.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let decoded_value =
                js_sys::decode_uri_component(value).map_or_else(|_| value.to_string(), Into::into);
            params.insert(key.to_string(), decoded_value);
        }
    }
    params
}

/// Updates the browser URL with the given query parameters without triggering navigation.
/// Empty values are omitted from the URL.
pub fn write_query_params(params: &[(&str, &str)]) {
    let Some(win) = window() else {
        return;
    };
    let Some(history) = win.history().ok() else {
        return;
    };

    let query_parts: Vec<String> = params
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| {
            let encoded = js_sys::encode_uri_component(v);
            format!("{k}={encoded}")
        })
        .collect();

    let pathname = win
        .location()
        .pathname()
        .unwrap_or_else(|_| "/".to_string());

    let new_url = if query_parts.is_empty() {
        pathname
    } else {
        format!("{pathname}?{}", query_parts.join("&"))
    };

    let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&new_url));
}
