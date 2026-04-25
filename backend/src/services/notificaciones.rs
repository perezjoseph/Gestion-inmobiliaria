use chrono::Utc;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::entities::{contrato, inquilino, pago, propiedad};
use crate::errors::AppError;
use crate::models::notificacion::PagoVencido;

pub async fn listar_pagos_vencidos(db: &DatabaseConnection) -> Result<Vec<PagoVencido>, AppError> {
    let today = Utc::now().date_naive();

    let pagos = pago::Entity::find()
        .filter(pago::Column::Estado.eq("pendiente"))
        .filter(pago::Column::FechaVencimiento.lt(today))
        .all(db)
        .await?;

    if pagos.is_empty() {
        return Ok(vec![]);
    }

    let contrato_ids: Vec<uuid::Uuid> = pagos.iter().map(|p| p.contrato_id).collect();
    let contratos = contrato::Entity::find()
        .filter(contrato::Column::Id.is_in(contrato_ids))
        .all(db)
        .await?;

    let propiedad_ids: Vec<uuid::Uuid> = contratos.iter().map(|c| c.propiedad_id).collect();
    let inquilino_ids: Vec<uuid::Uuid> = contratos.iter().map(|c| c.inquilino_id).collect();

    let propiedades = propiedad::Entity::find()
        .filter(propiedad::Column::Id.is_in(propiedad_ids))
        .all(db)
        .await?;

    let inquilinos = inquilino::Entity::find()
        .filter(inquilino::Column::Id.is_in(inquilino_ids))
        .all(db)
        .await?;

    let mut results: Vec<PagoVencido> = pagos
        .iter()
        .filter_map(|p| {
            let contrato_model = contratos.iter().find(|c| c.id == p.contrato_id)?;

            let prop = propiedades
                .iter()
                .find(|pr| pr.id == contrato_model.propiedad_id);

            let inq = inquilinos
                .iter()
                .find(|i| i.id == contrato_model.inquilino_id);

            let dias_vencido = (today - p.fecha_vencimiento).num_days();

            Some(PagoVencido {
                pago_id: p.id,
                propiedad_titulo: prop.map(|pr| pr.titulo.clone()).unwrap_or_default(),
                inquilino_nombre: inq.map(|i| i.nombre.clone()).unwrap_or_default(),
                inquilino_apellido: inq.map(|i| i.apellido.clone()).unwrap_or_default(),
                monto: p.monto,
                moneda: p.moneda.clone(),
                dias_vencido,
            })
        })
        .collect();

    results.sort_by_key(|b| std::cmp::Reverse(b.dias_vencido));

    Ok(results)
}
