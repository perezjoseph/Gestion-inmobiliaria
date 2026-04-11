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

fn humanize_error(status: u16, raw: &str) -> String {
    #[derive(serde::Deserialize)]
    struct ApiErr {
        #[serde(default)]
        error: String,
        #[serde(default)]
        message: String,
    }

    if let Ok(parsed) = serde_json::from_str::<ApiErr>(raw)
        && !parsed.message.is_empty()
    {
        let msg = &parsed.message;
        if msg.contains("duplicate key") || parsed.error == "conflict" {
            if msg.contains("cedula") {
                return "Esta cédula ya está registrada en el sistema.".into();
            }
            if msg.contains("email") || msg.contains("correo") {
                return "Este correo electrónico ya está en uso.".into();
            }
            return "Este registro ya existe. Verifique los datos e intente de nuevo.".into();
        }
        if msg.contains("superpone") || msg.contains("overlap") || msg.contains("solapamiento") {
            return "El contrato se superpone con otro contrato activo para esta propiedad.".into();
        }
        if parsed.error == "validation" {
            return parsed.message;
        }
        if parsed.error == "not_found" {
            return "El registro solicitado no fue encontrado.".into();
        }
        if parsed.error == "forbidden" {
            return "No tiene permisos para realizar esta acción.".into();
        }
        return parsed.message;
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
