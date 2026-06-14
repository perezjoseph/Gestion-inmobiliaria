use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::entities::cache_dgii;
use crate::errors::AppError;
use crate::models::dgii::{
    DgiiConsultaResponse, DgiiNombreItem, DgiiNombreResponse, MegaplusApiResponse,
};

fn normalize_rnc(rnc: &str) -> String {
    rnc.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
}

fn validate_rnc(normalized: &str) -> Result<(), AppError> {
    let len = normalized.len();
    if len != 9 && len != 11 {
        return Err(AppError::Validation(
            "RNC debe tener 9 dígitos o cédula debe tener 11 dígitos".to_string(),
        ));
    }
    Ok(())
}

fn get_base_url() -> String {
    std::env::var("DGII_API_BASE_URL").unwrap_or_else(|_| "https://rnc.megaplus.com.do".to_string())
}

fn build_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error creando HTTP client: {e}")))
}

pub async fn consultar_rnc(
    db: &DatabaseConnection,
    organizacion_id: Uuid,
    rnc: &str,
) -> Result<DgiiConsultaResponse, AppError> {
    let normalized = normalize_rnc(rnc);
    validate_rnc(&normalized)?;

    let now = Utc::now().fixed_offset();

    let cached = cache_dgii::Entity::find()
        .filter(cache_dgii::Column::CedulaRnc.eq(&normalized))
        .filter(cache_dgii::Column::OrganizacionId.eq(organizacion_id))
        .filter(cache_dgii::Column::ExpiresAt.gt(now))
        .one(db)
        .await?;

    if let Some(entry) = cached {
        return Ok(DgiiConsultaResponse {
            cedula_rnc: entry.cedula_rnc,
            nombre_razon_social: entry.nombre_razon_social,
            nombre_comercial: entry.nombre_comercial,
            estado: entry.estado,
            regimen_de_pagos: entry.regimen_de_pagos,
            actividad_economica: entry.actividad_economica,
            cached: true,
        });
    }

    let base_url = get_base_url();
    let client = build_client()?;
    let url = format!("{base_url}/api/rnc?rnc={normalized}");

    let api_result = client.get(&url).send().await;

    match api_result {
        Ok(response) => {
            if !response.status().is_success() {
                return try_stale_cache_or_error(db, organizacion_id, &normalized).await;
            }

            let api_response: MegaplusApiResponse = response.json().await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Error parseando respuesta DGII: {e}"))
            })?;

            let data = api_response.data.ok_or_else(|| {
                AppError::NotFound("RNC/cédula no encontrado en DGII".to_string())
            })?;

            let cedula_rnc = data
                .get("cedula_rnc")
                .and_then(|v| v.as_str())
                .unwrap_or(&normalized)
                .to_string();
            let nombre_razon_social = data
                .get("nombre_razon_social")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let nombre_comercial = data
                .get("nombre_comercial")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string);
            let estado = data
                .get("estado")
                .and_then(|v| v.as_str())
                .unwrap_or("DESCONOCIDO")
                .to_string();
            let regimen_de_pagos = data
                .get("regimen_de_pagos")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string);
            let actividad_economica = data
                .get("actividad_economica")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string);

            let cached_at = Utc::now().fixed_offset();
            let expires_at = cached_at + chrono::Duration::hours(24);

            let existing = cache_dgii::Entity::find()
                .filter(cache_dgii::Column::CedulaRnc.eq(&normalized))
                .filter(cache_dgii::Column::OrganizacionId.eq(organizacion_id))
                .one(db)
                .await?;

            if let Some(existing_entry) = existing {
                let mut active: cache_dgii::ActiveModel = existing_entry.into();
                active.nombre_razon_social = Set(nombre_razon_social.clone());
                active.nombre_comercial = Set(nombre_comercial.clone());
                active.estado = Set(estado.clone());
                active.regimen_de_pagos = Set(regimen_de_pagos.clone());
                active.actividad_economica = Set(actividad_economica.clone());
                active.raw_response = Set(data.clone());
                active.cached_at = Set(cached_at);
                active.expires_at = Set(expires_at);
                active.updated_at = Set(cached_at);
                active.update(db).await?;
            } else {
                let new_entry = cache_dgii::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    cedula_rnc: Set(normalized.clone()),
                    nombre_razon_social: Set(nombre_razon_social.clone()),
                    nombre_comercial: Set(nombre_comercial.clone()),
                    estado: Set(estado.clone()),
                    regimen_de_pagos: Set(regimen_de_pagos.clone()),
                    actividad_economica: Set(actividad_economica.clone()),
                    raw_response: Set(data.clone()),
                    organizacion_id: Set(organizacion_id),
                    cached_at: Set(cached_at),
                    expires_at: Set(expires_at),
                    created_at: Set(cached_at),
                    updated_at: Set(cached_at),
                };
                new_entry.insert(db).await?;
            }

            Ok(DgiiConsultaResponse {
                cedula_rnc,
                nombre_razon_social,
                nombre_comercial,
                estado,
                regimen_de_pagos,
                actividad_economica,
                cached: false,
            })
        }
        Err(e) => {
            tracing::warn!("Error conectando con DGII API: {e}");
            try_stale_cache_or_error(db, organizacion_id, &normalized).await
        }
    }
}

async fn try_stale_cache_or_error(
    db: &DatabaseConnection,
    organizacion_id: Uuid,
    normalized_rnc: &str,
) -> Result<DgiiConsultaResponse, AppError> {
    let stale = cache_dgii::Entity::find()
        .filter(cache_dgii::Column::CedulaRnc.eq(normalized_rnc))
        .filter(cache_dgii::Column::OrganizacionId.eq(organizacion_id))
        .one(db)
        .await?;

    match stale {
        Some(entry) => {
            tracing::warn!(
                rnc = %normalized_rnc,
                "Usando cache DGII expirado por fallo de API"
            );
            Ok(DgiiConsultaResponse {
                cedula_rnc: entry.cedula_rnc,
                nombre_razon_social: entry.nombre_razon_social,
                nombre_comercial: entry.nombre_comercial,
                estado: entry.estado,
                regimen_de_pagos: entry.regimen_de_pagos,
                actividad_economica: entry.actividad_economica,
                cached: true,
            })
        }
        None => Err(AppError::Internal(anyhow::anyhow!(
            "DGII API no disponible y no hay cache para RNC {normalized_rnc}"
        ))),
    }
}

pub async fn consultar_nombre(buscar: &str) -> Result<DgiiNombreResponse, AppError> {
    let base_url = get_base_url();
    let client = build_client()?;
    let url = format!("{base_url}/api/rnc?name={buscar}");

    let response =
        client.get(&url).send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Error conectando con DGII API: {e}"))
        })?;

    if !response.status().is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "DGII API respondió con status {}",
            response.status()
        )));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Error parseando respuesta DGII: {e}")))?;

    let items = body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let cedula_rnc = item.get("cedula_rnc")?.as_str()?.to_string();
                    let nombre_razon_social =
                        item.get("nombre_razon_social")?.as_str()?.to_string();
                    let nombre_comercial = item
                        .get("nombre_comercial")
                        .and_then(|v| v.as_str())
                        .map(std::string::ToString::to_string);
                    let estado = item.get("estado")?.as_str()?.to_string();
                    Some(DgiiNombreItem {
                        cedula_rnc,
                        nombre_razon_social,
                        nombre_comercial,
                        estado,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(DgiiNombreResponse { resultados: items })
}

pub async fn invalidar_cache(
    db: &DatabaseConnection,
    organizacion_id: Uuid,
    rnc: &str,
) -> Result<(), AppError> {
    let normalized = normalize_rnc(rnc);

    cache_dgii::Entity::delete_many()
        .filter(cache_dgii::Column::CedulaRnc.eq(&normalized))
        .filter(cache_dgii::Column::OrganizacionId.eq(organizacion_id))
        .exec(db)
        .await?;

    Ok(())
}

pub async fn validar_cedula_inquilino<C: sea_orm::ConnectionTrait>(
    db: &C,
    organizacion_id: Uuid,
    cedula: &str,
) -> Option<DgiiConsultaResponse> {
    let normalized = normalize_rnc(cedula);
    if validate_rnc(&normalized).is_err() {
        return None;
    }

    let now = Utc::now().fixed_offset();

    let cached = cache_dgii::Entity::find()
        .filter(cache_dgii::Column::CedulaRnc.eq(&normalized))
        .filter(cache_dgii::Column::OrganizacionId.eq(organizacion_id))
        .filter(cache_dgii::Column::ExpiresAt.gt(now))
        .one(db)
        .await
        .ok()?;

    cached.map(|entry| DgiiConsultaResponse {
        cedula_rnc: entry.cedula_rnc,
        nombre_razon_social: entry.nombre_razon_social,
        nombre_comercial: entry.nombre_comercial,
        estado: entry.estado,
        regimen_de_pagos: entry.regimen_de_pagos,
        actividad_economica: entry.actividad_economica,
        cached: true,
    })
}
