use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::propiedad;
use crate::errors::AppError;
use crate::models::PaginatedResponse;
use crate::models::propiedad::{
    CreatePropiedadRequest, PropiedadListQuery, PropiedadResponse, UpdatePropiedadRequest,
};

impl From<propiedad::Model> for PropiedadResponse {
    fn from(m: propiedad::Model) -> Self {
        Self {
            id: m.id,
            titulo: m.titulo,
            descripcion: m.descripcion,
            direccion: m.direccion,
            ciudad: m.ciudad,
            provincia: m.provincia,
            tipo_propiedad: m.tipo_propiedad,
            habitaciones: m.habitaciones,
            banos: m.banos,
            area_m2: m.area_m2,
            precio: m.precio,
            moneda: m.moneda,
            estado: m.estado,
            imagenes: m.imagenes,
            created_at: m.created_at.into(),
            updated_at: m.updated_at.into(),
        }
    }
}

pub async fn create(
    db: &DatabaseConnection,
    input: CreatePropiedadRequest,
) -> Result<PropiedadResponse, AppError> {
    let now = Utc::now().into();
    let id = Uuid::new_v4();

    let model = propiedad::ActiveModel {
        id: Set(id),
        titulo: Set(input.titulo),
        descripcion: Set(input.descripcion),
        direccion: Set(input.direccion),
        ciudad: Set(input.ciudad),
        provincia: Set(input.provincia),
        tipo_propiedad: Set(input.tipo_propiedad),
        habitaciones: Set(input.habitaciones),
        banos: Set(input.banos),
        area_m2: Set(input.area_m2),
        precio: Set(input.precio),
        moneda: Set(input.moneda.unwrap_or_else(|| "DOP".to_string())),
        estado: Set(input.estado.unwrap_or_else(|| "disponible".to_string())),
        imagenes: Set(input.imagenes.or(Some(serde_json::json!([])))),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let record = model.insert(db).await?;
    Ok(PropiedadResponse::from(record))
}

pub async fn get_by_id(db: &DatabaseConnection, id: Uuid) -> Result<PropiedadResponse, AppError> {
    let record = propiedad::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;
    Ok(PropiedadResponse::from(record))
}

pub async fn list(
    db: &DatabaseConnection,
    query: PropiedadListQuery,
) -> Result<PaginatedResponse<PropiedadResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).max(1);

    let mut select = propiedad::Entity::find();

    if let Some(ref ciudad) = query.ciudad {
        select = select.filter(propiedad::Column::Ciudad.eq(ciudad));
    }
    if let Some(ref provincia) = query.provincia {
        select = select.filter(propiedad::Column::Provincia.eq(provincia));
    }
    if let Some(ref tipo) = query.tipo_propiedad {
        select = select.filter(propiedad::Column::TipoPropiedad.eq(tipo));
    }
    if let Some(ref estado) = query.estado {
        select = select.filter(propiedad::Column::Estado.eq(estado));
    }
    if let Some(precio_min) = query.precio_min {
        select = select.filter(propiedad::Column::Precio.gte(precio_min));
    }
    if let Some(precio_max) = query.precio_max {
        select = select.filter(propiedad::Column::Precio.lte(precio_max));
    }

    let paginator = select
        .order_by_desc(propiedad::Column::CreatedAt)
        .paginate(db, per_page);

    let total = paginator.num_items().await?;
    let records = paginator.fetch_page(page - 1).await?;

    Ok(PaginatedResponse {
        data: records.into_iter().map(PropiedadResponse::from).collect(),
        total,
        page,
        per_page,
    })
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdatePropiedadRequest,
) -> Result<PropiedadResponse, AppError> {
    let existing = propiedad::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    let mut active: propiedad::ActiveModel = existing.into();

    if let Some(titulo) = input.titulo {
        active.titulo = Set(titulo);
    }
    if let Some(descripcion) = input.descripcion {
        active.descripcion = Set(Some(descripcion));
    }
    if let Some(direccion) = input.direccion {
        active.direccion = Set(direccion);
    }
    if let Some(ciudad) = input.ciudad {
        active.ciudad = Set(ciudad);
    }
    if let Some(provincia) = input.provincia {
        active.provincia = Set(provincia);
    }
    if let Some(tipo_propiedad) = input.tipo_propiedad {
        active.tipo_propiedad = Set(tipo_propiedad);
    }
    if let Some(habitaciones) = input.habitaciones {
        active.habitaciones = Set(Some(habitaciones));
    }
    if let Some(banos) = input.banos {
        active.banos = Set(Some(banos));
    }
    if let Some(area_m2) = input.area_m2 {
        active.area_m2 = Set(Some(area_m2));
    }
    if let Some(precio) = input.precio {
        active.precio = Set(precio);
    }
    if let Some(moneda) = input.moneda {
        active.moneda = Set(moneda);
    }
    if let Some(estado) = input.estado {
        active.estado = Set(estado);
    }
    if let Some(imagenes) = input.imagenes {
        active.imagenes = Set(Some(imagenes));
    }

    active.updated_at = Set(Utc::now().into());

    let updated = active.update(db).await?;
    Ok(PropiedadResponse::from(updated))
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<(), AppError> {
    let existing = propiedad::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Propiedad no encontrada".to_string()))?;

    propiedad::Entity::delete_by_id(existing.id)
        .exec(db)
        .await?;

    Ok(())
}
