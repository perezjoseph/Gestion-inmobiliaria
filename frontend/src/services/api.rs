use gloo_net::http::{Request, RequestBuilder};
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::JsCast;
use web_sys::window;

pub const BASE_URL: &str = "/api/v1";
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

#[derive(serde::Deserialize)]
struct ApiErr {
    #[serde(default)]
    error: String,
    #[serde(default)]
    message: String,
}

#[allow(clippy::unnecessary_wraps)]
fn humanize_duplicate_error(msg: &str) -> Option<String> {
    if msg.contains("cedula") {
        return Some("Esta cédula ya está registrada en el sistema.".into());
    }
    if msg.contains("email") || msg.contains("correo") {
        return Some("Este correo electrónico ya está en uso.".into());
    }
    Some("Este registro ya existe. Verifique los datos e intente de nuevo.".into())
}

fn humanize_parsed_error(parsed: &ApiErr) -> Option<String> {
    let msg = &parsed.message;
    if msg.contains("duplicate key") || parsed.error == "conflict" {
        return humanize_duplicate_error(msg);
    }
    if msg.contains("superpone") || msg.contains("overlap") || msg.contains("solapamiento") {
        return Some(
            "El contrato se superpone con otro contrato activo para esta propiedad.".into(),
        );
    }
    if parsed.error == "validation" {
        return Some(parsed.message.clone());
    }
    if parsed.error == "not_found" {
        return Some("El registro solicitado no fue encontrado.".into());
    }
    if parsed.error == "forbidden" {
        return Some("No tiene permisos para realizar esta acción.".into());
    }
    Some(parsed.message.clone())
}

fn humanize_error(status: u16, raw: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<ApiErr>(raw)
        && !parsed.message.is_empty()
        && let Some(msg) = humanize_parsed_error(&parsed)
    {
        return msg;
    }

    match status {
        400 => "Solicitud inválida. Verifique los datos enviados.".into(),
        403 => "No tiene permisos para realizar esta acción.".into(),
        404 => "El registro solicitado no fue encontrado.".into(),
        409 => "Este registro ya existe. Verifique los datos e intente de nuevo.".into(),
        422 => "Datos inválidos. Revise los campos e intente de nuevo.".into(),
        500 => "Error interno del servidor. Intente de nuevo más tarde.".into(),
        _ => format!("Error inesperado (código {status}). Intente de nuevo."),
    }
}

#[allow(clippy::future_not_send)]
async fn handle_response(
    response: gloo_net::http::Response,
) -> Result<gloo_net::http::Response, String> {
    if response.status() == 401 {
        clear_token_and_redirect();
        return Err("Sesión expirada. Redirigiendo al inicio de sesión.".into());
    }

    if !response.ok() {
        let status = response.status();
        let text = response
            .text()
            .await
            .unwrap_or_else(|_| "Error desconocido".into());
        return Err(humanize_error(status, &text));
    }

    Ok(response)
}

#[allow(clippy::future_not_send)]
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

#[allow(clippy::future_not_send)]
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

#[allow(clippy::future_not_send)]
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

#[allow(clippy::future_not_send)]
pub async fn api_delete(path: &str) -> Result<(), String> {
    let url = format!("{BASE_URL}{path}");
    let response = apply_auth(Request::delete(&url))
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    handle_response(response).await?;
    Ok(())
}

/// Authenticated binary download: fetches the URL with the JWT header,
/// creates a blob, and triggers a browser download with the given filename.
#[allow(clippy::future_not_send)]
pub async fn api_download(path: &str, filename: &str) -> Result<(), String> {
    let url = format!("{BASE_URL}{path}");
    let response = apply_auth(Request::get(&url))
        .send()
        .await
        .map_err(|e| format!("Error de red: {e}"))?;
    let response = handle_response(response).await?;

    let bytes = response
        .binary()
        .await
        .map_err(|e| format!("Error al leer respuesta: {e}"))?;

    let uint8 = js_sys::Uint8Array::from(bytes.as_slice());
    let array = js_sys::Array::new();
    array.push(&uint8.buffer());

    let blob = web_sys::Blob::new_with_u8_array_sequence(&array)
        .map_err(|_| "Error al crear blob")?;

    let blob_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_| "Error al crear URL de descarga")?;

    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        let a: web_sys::HtmlElement = doc
            .create_element("a")
            .map_err(|_| "Error al crear enlace")?
            .unchecked_into();
        a.set_attribute("href", &blob_url)
            .map_err(|_| "Error al configurar enlace")?;
        a.set_attribute("download", filename)
            .map_err(|_| "Error al configurar enlace")?;
        a.click();
    }

    let _ = web_sys::Url::revoke_object_url(&blob_url);
    Ok(())
}

use crate::types::ocr::ConfirmPreviewRequest;

#[allow(dead_code, clippy::future_not_send)]
pub async fn confirmar_preview(
    request: &ConfirmPreviewRequest,
) -> Result<serde_json::Value, String> {
    api_post("/importar/ocr/confirmar", request).await
}

#[allow(dead_code, clippy::future_not_send)]
pub async fn descartar_preview(preview_id: &str) -> Result<(), String> {
    api_delete(&format!("/importar/ocr/preview/{preview_id}")).await
}
