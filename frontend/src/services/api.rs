use gloo_net::http::{Request, RequestBuilder};
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

fn apply_auth(builder: RequestBuilder) -> RequestBuilder {
    if let Some(token) = get_token() {
        builder.header("Authorization", &format!("Bearer {token}"))
    } else {
        builder
    }
}

async fn handle_response(
    response: gloo_net::http::Response,
) -> Result<gloo_net::http::Response, String> {
    if response.status() == 401 {
        clear_token_and_redirect();
        return Err("Sesión expirada. Redirigiendo al inicio de sesión.".into());
    }

    if !response.ok() {
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Error desconocido".into());
        return Err(text);
    }

    Ok(response)
}

pub async fn api_get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let url = format!("{BASE_URL}{path}");
    let response = apply_auth(Request::get(&url))
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    let response = handle_response(response).await?;
    response
        .json::<T>()
        .await
        .map_err(|e| format!("Error al procesar respuesta: {e}"))
}

pub async fn api_post<T: DeserializeOwned, B: Serialize>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    let url = format!("{BASE_URL}{path}");
    let json =
        serde_json::to_string(body).map_err(|e| format!("Error al serializar datos: {e}"))?;
    let response = apply_auth(Request::post(&url))
        .header("Content-Type", "application/json")
        .body(json)
        .map_err(|e| format!("Error al serializar datos: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    let response = handle_response(response).await?;
    response
        .json::<T>()
        .await
        .map_err(|e| format!("Error al procesar respuesta: {e}"))
}

pub async fn api_put<T: DeserializeOwned, B: Serialize>(path: &str, body: &B) -> Result<T, String> {
    let url = format!("{BASE_URL}{path}");
    let json =
        serde_json::to_string(body).map_err(|e| format!("Error al serializar datos: {e}"))?;
    let response = apply_auth(Request::put(&url))
        .header("Content-Type", "application/json")
        .body(json)
        .map_err(|e| format!("Error al serializar datos: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    let response = handle_response(response).await?;
    response
        .json::<T>()
        .await
        .map_err(|e| format!("Error al procesar respuesta: {e}"))
}

pub async fn api_delete(path: &str) -> Result<(), String> {
    let url = format!("{BASE_URL}{path}");
    let response = apply_auth(Request::delete(&url))
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    handle_response(response).await?;
    Ok(())
}
