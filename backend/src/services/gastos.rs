use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{gasto, propiedad, unidad};
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::gasto::{
    CreateGastoRequest, GastoListQuery, GastoResponse, ResumenCategoriaRow, ResumenCategoriasQuery,
    UpdateGastoRequest,
};
use crate::services::auditoria::{self, CreateAuditoriaEntry};
use crate::services::servicios_publicos::PROVEEDORES_SERVICIO;
use crate::services::validation::{MONEDAS, validate_enum};

pub const CATEGORIAS_GASTO: &[&str] = &[
    "mantenimiento",
    "servicios",
    "impuestos",
    "seguro",
    "administracion",
    "otros",
];
pub const ESTADOS_GASTO: &[&str] = &["pendiente", "pagado", "cancelado"];

impl From<gasto::Model> for GastoResponse {
    fn from(m: gasto::Model) -> Self {
        Self {
            id: m.id,
            propiedad_id: m.propiedad_id,
            unidad_id: m.unidad_id,
            numero_unidad: None,
            categoria: m.categoria,
            descripcion: m.descripcion,
            monto: m.monto,
            moneda: m.moneda,
            fecha_gasto: m.fecha_gasto,
            estado: m.estado,
            proveedor: m.proveedor,
            numero_factura: m.numero_factura,
            notas: m.notas,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
            nic_contrato: m.nic_contrato,
            proveedor_servicio: m.proveedor_servicio,
            consumo: m.consumo,
            unidad_consumo: m.unidad_consumo,
            periodo_desde: m.periodo_desde,
            periodo_hasta: m.periodo_hasta,
            numero_cuenta: m.numero_cuenta,
            periodo_inicio: m.periodo_inicio,
            periodo_fin: m.periodo_fin,
        }
    }
}

impl GastoResponse {
    fn from_with_unidad(m: gasto::Model, unidad: Option<unidad::Model>) -> Self {
        let numero_unidad = unidad.map(|u| u.numero_unidad);
        let mut resp = Self::from(m);
        resp.numero_unidad = numero_unidad;
        resp
    }
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    input: CreateGastoRequest,
    usuario_id: Uuid,
    organizacion_id: Uuid,
) -> Result<GastoResponse, AppError> {
    if !CATEGORIAS_GASTO.contains(&input.categoria.as_str()) {
        return Err(AppError::Validation(
            "Categoría de gasto no válida".to_string(),
        ));
    }
    validate_enum("moneda", &input.moneda, MONEDAS)?;

    if input.monto <= Decimal::ZERO {
        return Err(AppError::Validation(
            "El monto debe ser mayor que cero".to_string(),
        ));
    }

    propiedad::Entity::find_by_id(input.propiedad_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    if let Some(unidad_id) = input.unidad_id {
        let unidad = unidad::Entity::find_by_id(unidad_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Unidad no encontrada".to_string()))?;
        if unidad.propiedad_id != input.propiedad_id {
            return Err(AppError::Validation(
                "La unidad no pertenece a la propiedad especificada".to_string(),
            ));
        }
    }

    // Utility field validation when categoria == "servicios"
    if input.categoria == "servicios" {
        if input.proveedor_servicio.is_none() {
            return Err(AppError::Validation(
                "proveedor_servicio es requerido para gastos de servicio público".to_string(),
            ));
        }
        validate_enum(
            "proveedor_servicio",
            input.proveedor_servicio.as_deref().unwrap_or_default(),
            PROVEEDORES_SERVICIO,
        )?;
        if let Some(consumo) = input.consumo {
            if consumo <= Decimal::ZERO {
                return Err(AppError::Validation(
                    "El consumo debe ser mayor que cero".to_string(),
                ));
            }
        }
        if let (Some(desde), Some(hasta)) = (input.periodo_desde, input.periodo_hasta) {
            if desde >= hasta {
                return Err(AppError::Validation(
                    "periodo_desde debe ser anterior a periodo_hasta".to_string(),
                ));
            }
        }
    }

    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = gasto::ActiveModel {
        id: Set(id),
        propiedad_id: Set(input.propiedad_id),
        unidad_id: Set(input.unidad_id),
        categoria: Set(input.categoria),
        descripcion: Set(input.descripcion),
        monto: Set(input.monto),
        moneda: Set(input.moneda),
        fecha_gasto: Set(input.fecha_gasto),
        estado: Set("pendiente".to_string()),
        proveedor: Set(input.proveedor),
        numero_factura: Set(input.numero_factura),
        notas: Set(input.notas),
        organizacion_id: Set(organizacion_id),
        nic_contrato: Set(input.nic_contrato),
        proveedor_servicio: Set(input.proveedor_servicio),
        consumo: Set(input.consumo),
        unidad_consumo: Set(input.unidad_consumo),
        periodo_desde: Set(input.periodo_desde),
        periodo_hasta: Set(input.periodo_hasta),
        numero_cuenta: Set(input.numero_cuenta),
        periodo_inicio: Set(input.periodo_inicio),
        periodo_fin: Set(input.periodo_fin),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "gasto".to_string(),
            entity_id: id,
            accion: "crear".to_string(),
            cambios: serde_json::json!(GastoResponse::from(record.clone())),
        },
    )
    .await;

    // Best-effort anomaly detection for utility gastos
    if record.categoria == "servicios" {
        if let Err(e) =
            super::servicios_publicos::verificar_consumo_anormal(db, &record, organizacion_id).await
        {
            tracing::warn!(gasto_id = %id, error = %e, "Error en verificación de consumo anormal");
        }
    }

    Ok(GastoResponse::from(record))
}

pub async fn get_by_id(
    db: &DatabaseConnection,
    org_id: Uuid,
    id: Uuid,
) -> Result<GastoResponse, AppError> {
    let (record, unidad) = gasto::Entity::find_by_id(id)
        .filter(gasto::Column::OrganizacionId.eq(org_id))
        .find_also_related(unidad::Entity)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Gasto no encontrado".to_string()))?;
    Ok(GastoResponse::from_with_unidad(record, unidad))
}

pub async fn list(
    db: &DatabaseConnection,
    org_id: Uuid,
    query: GastoListQuery,
) -> Result<PaginatedResponse<GastoResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);

    if let (Some(desde), Some(hasta)) = (query.fecha_desde, query.fecha_hasta) {
        if desde > hasta {
            return Err(AppError::BadRequest(
                "fecha_desde no puede ser posterior a fecha_hasta".to_string(),
            ));
        }
    }

    let mut select = gasto::Entity::find().filter(gasto::Column::OrganizacionId.eq(org_id));

    if let Some(propiedad_id) = query.propiedad_id {
        select = select.filter(gasto::Column::PropiedadId.eq(propiedad_id));
    }
    if let Some(unidad_id) = query.unidad_id {
        select = select.filter(gasto::Column::UnidadId.eq(unidad_id));
    }
    if let Some(ref categoria) = query.categoria {
        select = select.filter(gasto::Column::Categoria.eq(categoria));
    }
    if let Some(ref estado) = query.estado {
        select = select.filter(gasto::Column::Estado.eq(estado));
    }
    if let Some(fecha_desde) = query.fecha_desde {
        select = select.filter(gasto::Column::FechaGasto.gte(fecha_desde));
    }
    if let Some(fecha_hasta) = query.fecha_hasta {
        select = select.filter(gasto::Column::FechaGasto.lte(fecha_hasta));
    }
    if let Some(ref proveedor_servicio) = query.proveedor_servicio {
        select = select.filter(gasto::Column::ProveedorServicio.eq(proveedor_servicio));
    }
    if let Some(periodo_desde) = query.periodo_desde {
        select = select.filter(gasto::Column::PeriodoDesde.gte(periodo_desde));
    }
    if let Some(periodo_hasta) = query.periodo_hasta {
        select = select.filter(gasto::Column::PeriodoHasta.lte(periodo_hasta));
    }

    let select = select
        .find_also_related(unidad::Entity)
        .order_by_desc(gasto::Column::FechaGasto);

    let paginator = select.paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records
            .into_iter()
            .map(|(g, u)| GastoResponse::from_with_unidad(g, u))
            .collect(),
        total,
        page,
        per_page,
    })
}

fn validate_gasto_update(input: &UpdateGastoRequest) -> Result<(), AppError> {
    if let Some(ref categoria) = input.categoria {
        if !CATEGORIAS_GASTO.contains(&categoria.as_str()) {
            return Err(AppError::Validation(
                "Categoría de gasto no válida".to_string(),
            ));
        }
    }
    if let Some(ref moneda) = input.moneda {
        validate_enum("moneda", moneda, MONEDAS)?;
    }
    if let Some(ref estado) = input.estado {
        validate_enum("estado", estado, ESTADOS_GASTO)?;
    }
    if let Some(ref monto) = input.monto
        && *monto <= Decimal::ZERO
    {
        return Err(AppError::Validation(
            "El monto debe ser mayor que cero".to_string(),
        ));
    }
    Ok(())
}

pub async fn update<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    id: Uuid,
    input: UpdateGastoRequest,
    usuario_id: Uuid,
) -> Result<GastoResponse, AppError> {
    validate_gasto_update(&input)?;

    let existing = gasto::Entity::find_by_id(id)
        .filter(gasto::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Gasto no encontrado".to_string()))?;

    let propiedad_id = existing.propiedad_id;

    if let Some(unidad_id) = input.unidad_id {
        let unidad = unidad::Entity::find_by_id(unidad_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Unidad no encontrada".to_string()))?;
        if unidad.propiedad_id != propiedad_id {
            return Err(AppError::Validation(
                "La unidad no pertenece a la propiedad especificada".to_string(),
            ));
        }
    }

    let mut active: gasto::ActiveModel = existing.into();

    if let Some(categoria) = input.categoria {
        active.categoria = Set(categoria);
    }
    if let Some(descripcion) = input.descripcion {
        active.descripcion = Set(descripcion);
    }
    if let Some(monto) = input.monto {
        active.monto = Set(monto);
    }
    if let Some(moneda) = input.moneda {
        active.moneda = Set(moneda);
    }
    if let Some(fecha_gasto) = input.fecha_gasto {
        active.fecha_gasto = Set(fecha_gasto);
    }
    if let Some(unidad_id) = input.unidad_id {
        active.unidad_id = Set(Some(unidad_id));
    }
    if let Some(proveedor) = input.proveedor {
        active.proveedor = Set(Some(proveedor));
    }
    if let Some(numero_factura) = input.numero_factura {
        active.numero_factura = Set(Some(numero_factura));
    }
    if let Some(estado) = input.estado {
        active.estado = Set(estado);
    }
    if let Some(notas) = input.notas {
        active.notas = Set(Some(notas));
    }

    active.updated_at = Set(Utc::now().into());

    let updated = active.update(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "gasto".to_string(),
            entity_id: id,
            accion: "actualizar".to_string(),
            cambios: serde_json::json!(GastoResponse::from(updated.clone())),
        },
    )
    .await;

    Ok(GastoResponse::from(updated))
}

pub async fn delete<C: ConnectionTrait>(
    db: &C,
    org_id: Uuid,
    id: Uuid,
    usuario_id: Uuid,
) -> Result<(), AppError> {
    let existing = gasto::Entity::find_by_id(id)
        .filter(gasto::Column::OrganizacionId.eq(org_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Gasto no encontrado".to_string()))?;

    let active: gasto::ActiveModel = existing.into();
    active.delete(db).await?;

    auditoria::registrar_best_effort(
        db,
        CreateAuditoriaEntry {
            usuario_id,
            entity_type: "gasto".to_string(),
            entity_id: id,
            accion: "eliminar".to_string(),
            cambios: serde_json::json!({ "id": id }),
        },
    )
    .await;

    Ok(())
}

#[derive(Debug, FromQueryResult)]
struct CategoryAggRow {
    categoria: String,
    total: Decimal,
    cantidad: i64,
}

pub async fn resumen_categorias(
    db: &DatabaseConnection,
    org_id: Uuid,
    query: ResumenCategoriasQuery,
) -> Result<Vec<ResumenCategoriaRow>, AppError> {
    use sea_orm::{QuerySelect, sea_query::Expr};

    let mut select = gasto::Entity::find()
        .filter(gasto::Column::OrganizacionId.eq(org_id))
        .filter(gasto::Column::PropiedadId.eq(query.propiedad_id))
        .filter(gasto::Column::Estado.eq("pagado"));

    if let Some(fecha_desde) = query.fecha_desde {
        select = select.filter(gasto::Column::FechaGasto.gte(fecha_desde));
    }
    if let Some(fecha_hasta) = query.fecha_hasta {
        select = select.filter(gasto::Column::FechaGasto.lte(fecha_hasta));
    }

    let rows: Vec<CategoryAggRow> = select
        .select_only()
        .column(gasto::Column::Categoria)
        .column_as(Expr::col(gasto::Column::Monto).sum(), "total")
        .column_as(Expr::col(gasto::Column::Id).count(), "cantidad")
        .group_by(gasto::Column::Categoria)
        .order_by_desc(Expr::col(gasto::Column::Monto).sum())
        .into_model::<CategoryAggRow>()
        .all(db)
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| ResumenCategoriaRow {
            categoria: r.categoria,
            total: r.total,
            cantidad: r.cantidad as u64,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_model(categoria: &str, estado: &str) -> gasto::Model {
        gasto::Model {
            id: Uuid::new_v4(),
            propiedad_id: Uuid::new_v4(),
            unidad_id: None,
            categoria: categoria.to_string(),
            descripcion: "Test gasto".to_string(),
            monto: Decimal::new(10000, 2),
            moneda: "DOP".to_string(),
            fecha_gasto: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap_or_default(),
            estado: estado.to_string(),
            proveedor: Some("Proveedor ABC".to_string()),
            numero_factura: Some("FAC-001".to_string()),
            notas: Some("Nota de prueba".to_string()),
            organizacion_id: Uuid::new_v4(),
            nic_contrato: None,
            proveedor_servicio: None,
            consumo: None,
            unidad_consumo: None,
            periodo_desde: None,
            periodo_hasta: None,
            numero_cuenta: None,
            periodo_inicio: None,
            periodo_fin: None,
            created_at: Utc::now().fixed_offset(),
            updated_at: Utc::now().fixed_offset(),
        }
    }

    #[test]
    fn from_model_converts_all_fields() {
        let model = make_model("mantenimiento", "pendiente");
        let original_id = model.id;
        let original_propiedad_id = model.propiedad_id;

        let resp = GastoResponse::from(model);
        assert_eq!(resp.id, original_id);
        assert_eq!(resp.propiedad_id, original_propiedad_id);
        assert!(resp.unidad_id.is_none());
        assert_eq!(resp.categoria, "mantenimiento");
        assert_eq!(resp.descripcion, "Test gasto");
        assert_eq!(resp.monto, Decimal::new(10000, 2));
        assert_eq!(resp.moneda, "DOP");
        assert_eq!(
            resp.fecha_gasto,
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap_or_default()
        );
        assert_eq!(resp.estado, "pendiente");
        assert_eq!(resp.proveedor.as_deref(), Some("Proveedor ABC"));
        assert_eq!(resp.numero_factura.as_deref(), Some("FAC-001"));
        assert_eq!(resp.notas.as_deref(), Some("Nota de prueba"));
    }

    #[test]
    fn from_model_converts_created_at_to_utc() {
        let model = make_model("impuestos", "pagado");
        let resp = GastoResponse::from(model);
        assert_eq!(resp.created_at.timezone(), Utc);
        assert_eq!(resp.updated_at.timezone(), Utc);
    }

    #[test]
    fn from_model_with_unidad_id() {
        let mut model = make_model("seguro", "pendiente");
        let uid = Uuid::new_v4();
        model.unidad_id = Some(uid);
        let resp = GastoResponse::from(model);
        assert_eq!(resp.unidad_id, Some(uid));
    }

    #[test]
    fn from_model_with_none_optional_fields() {
        let mut model = make_model("servicios", "cancelado");
        model.proveedor = None;
        model.numero_factura = None;
        model.notas = None;
        let resp = GastoResponse::from(model);
        assert!(resp.proveedor.is_none());
        assert!(resp.numero_factura.is_none());
        assert!(resp.notas.is_none());
    }

    #[test]
    fn categorias_gasto_contains_expected_values() {
        assert!(CATEGORIAS_GASTO.contains(&"mantenimiento"));
        assert!(CATEGORIAS_GASTO.contains(&"servicios"));
        assert!(CATEGORIAS_GASTO.contains(&"impuestos"));
        assert!(CATEGORIAS_GASTO.contains(&"seguro"));
        assert!(CATEGORIAS_GASTO.contains(&"administracion"));
        assert!(CATEGORIAS_GASTO.contains(&"otros"));
        assert_eq!(CATEGORIAS_GASTO.len(), 6);
    }

    #[test]
    fn estados_gasto_contains_expected_values() {
        assert!(ESTADOS_GASTO.contains(&"pendiente"));
        assert!(ESTADOS_GASTO.contains(&"pagado"));
        assert!(ESTADOS_GASTO.contains(&"cancelado"));
        assert_eq!(ESTADOS_GASTO.len(), 3);
    }

    #[test]
    fn monedas_contains_expected_values() {
        assert!(MONEDAS.contains(&"DOP"));
        assert!(MONEDAS.contains(&"USD"));
        assert_eq!(MONEDAS.len(), 2);
    }

    #[test]
    fn categorias_gasto_rejects_invalid() {
        assert!(!CATEGORIAS_GASTO.contains(&"invalido"));
        assert!(!CATEGORIAS_GASTO.contains(&""));
        assert!(!CATEGORIAS_GASTO.contains(&"Mantenimiento"));
    }

    #[test]
    fn estados_gasto_rejects_invalid() {
        assert!(!ESTADOS_GASTO.contains(&"activo"));
        assert!(!ESTADOS_GASTO.contains(&""));
    }

    #[test]
    fn monedas_rejects_invalid() {
        assert!(!MONEDAS.contains(&"EUR"));
        assert!(!MONEDAS.contains(&""));
    }

    #[test]
    fn from_model_all_categorias() {
        for cat in CATEGORIAS_GASTO {
            let model = make_model(cat, "pendiente");
            let resp = GastoResponse::from(model);
            assert_eq!(resp.categoria, *cat);
        }
    }

    #[test]
    fn from_model_all_estados() {
        for estado in ESTADOS_GASTO {
            let model = make_model("mantenimiento", estado);
            let resp = GastoResponse::from(model);
            assert_eq!(resp.estado, *estado);
        }
    }
}
