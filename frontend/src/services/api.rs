use gloo_net::http::{Method, Request};
use serde::{Serialize, de::DeserializeOwned};
use web_sys::window;

pub const BASE_URL: &str = "http://localhost:3000/api";
const TOKEN_KEY: &str = "jwt_token";

fn get_token() -> Option<String> {
    window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item(TOKEN_KEY)
        .ok()
        .flatten()
}

fn clear_token_and_redirect() {
    if let Some(win) = window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let _ = storage.remove_item(TOKEN_KEY);
        }
        let _ = win.location().set_href("/");
    }
}

async fn send_request(
    method: Method,
    path: &str,
    body: Option<String>,
) -> Result<gloo_net::http::Response, String> {
    let url = format!("{BASE_URL}{path}");
    let mut builder = Request::new(&url);
    builder = match method {
        Method::GET => builder.method(Method::GET),
        Method::POST => builder.method(Method::POST),
        Method::PUT => builder.method(Method::PUT),
        Method::DELETE => builder.method(Method::DELETE),
        _ => builder.method(method),
    };

    if let Some(token) = get_token() {
        builder = builder.header("Authorization", &format!("Bearer {token}"));
    }
    if body.is_some() {
        builder = builder.header("Content-Type", "application/json");
    }

    let response: gloo_net::http::Response = match body {
        Some(json) => {
            builder
                .body(json)
                .map_err(|e| format!("Error al serializar datos: {e}"))?
                .send()
                .await
        }
        None => builder.send().await,
    }
    .map_err(|e| format!("Error de red: {e}"))?;

    if response.status() == 401 {
        clear_token_and_redirect();
        return Err("Sesión expirada. Redirigiendo al inicio de sesión.".into());
    }

    if !response.ok() {
        let text: String = response
            .text()
            .await
            .unwrap_or_else(|_| "Error desconocido".into());
        return Err(text);
    }

    Ok(response)
}

pub async fn api_get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    send_request(Method::GET, path, None)
        .await?
        .json::<T>()
        .await
        .map_err(|e| format!("Error al procesar respuesta: {e}"))
}

pub async fn api_post<T: DeserializeOwned, B: Serialize>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    let json =
        serde_json::to_string(body).map_err(|e| format!("Error al serializar datos: {e}"))?;
    send_request(Method::POST, path, Some(json))
        .await?
        .json::<T>()
        .await
        .map_err(|e| format!("Error al procesar respuesta: {e}"))
}

pub async fn api_put<T: DeserializeOwned, B: Serialize>(path: &str, body: &B) -> Result<T, String> {
    let json =
        serde_json::to_string(body).map_err(|e| format!("Error al serializar datos: {e}"))?;
    send_request(Method::PUT, path, Some(json))
        .await?
        .json::<T>()
        .await
        .map_err(|e| format!("Error al procesar respuesta: {e}"))
}

pub async fn api_delete(path: &str) -> Result<(), String> {
    send_request(Method::DELETE, path, None).await?;
    Ok(())
}
