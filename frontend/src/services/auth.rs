use web_sys::window;

use crate::services::api::api_post;
use crate::types::usuario::{LoginRequest, LoginResponse, RegisterRequest, User};

const TOKEN_KEY: &str = "jwt_token";

pub fn get_token() -> Option<String> {
    window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item(TOKEN_KEY)
        .ok()
        .flatten()
}

pub fn set_token(token: &str) {
    if let Some(win) = window()
        && let Ok(Some(storage)) = win.local_storage()
    {
        let _ = storage.set_item(TOKEN_KEY, token);
    }
}

pub fn clear_token() {
    if let Some(win) = window()
        && let Ok(Some(storage)) = win.local_storage()
    {
        let _ = storage.remove_item(TOKEN_KEY);
    }
}

pub fn is_authenticated() -> bool {
    get_token().is_some()
}

#[allow(dead_code)]
pub fn logout() {
    clear_token();
    if let Some(win) = window() {
        let _ = win.location().set_href("/");
    }
}

#[allow(clippy::future_not_send)]
pub async fn login(request: LoginRequest) -> Result<LoginResponse, String> {
    let response: LoginResponse = api_post("/auth/login", &request).await?;
    set_token(&response.token);
    Ok(response)
}

#[allow(clippy::future_not_send)]
pub async fn register(request: RegisterRequest) -> Result<User, String> {
    api_post("/auth/register", &request).await
}
