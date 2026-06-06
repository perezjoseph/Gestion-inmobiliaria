use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use crate::entities::{organizacion, pago, recibo_informal};
use crate::errors::AppError;

/// Create a `Recibo_Informal` for a cash payment by an informal organization.
///
/// Validates that:
/// - The pago exists and belongs to the given organizacion
/// - The organization has `tipo_fiscal = "informal"`
/// - The payment has `metodo_pago = "efectivo"`
///
/// Generates a unique sequential `referencia_interna` in "RI-NNNNNN" format
/// (e.g., RI-000001, RI-000002) scoped to the organization.
pub async fn crear_recibo_informal(
    db: &DatabaseConnection,
    pago_id: Uuid,
    organizacion_id: Uuid,
) -> Result<recibo_informal::Model, AppError> {
    // Fetch the organization
    let org = organizacion::Entity::find_by_id(organizacion_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Organización no encontrada".to_string()))?;

    // Validate tipo_fiscal is informal
    if org.tipo_fiscal != "informal" {
        return Err(AppError::Validation(
            "Recibos informales solo aplican para organizaciones con tipo_fiscal informal"
                .to_string(),
        ));
    }

    // Fetch the pago and verify it belongs to this organization
    let pago_model = pago::Entity::find_by_id(pago_id)
        .filter(pago::Column::OrganizacionId.eq(organizacion_id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;

    // Validate metodo_pago is efectivo
    let metodo = pago_model.metodo_pago.as_deref().unwrap_or("");
    if metodo != "efectivo" {
        return Err(AppError::Validation(
            "Recibos informales solo aplican para pagos en efectivo".to_string(),
        ));
    }

    // Generate the next referencia_interna for this organization
    let referencia = generar_siguiente_referencia(db, organizacion_id).await?;

    // Insert the recibo_informal record
    let now = chrono::Utc::now().into();
    let id = Uuid::new_v4();

    let active = recibo_informal::ActiveModel {
        id: Set(id),
        pago_id: Set(pago_id),
        referencia_interna: Set(referencia),
        organizacion_id: Set(organizacion_id),
        created_at: Set(now),
    };

    let model = active.insert(db).await?;
    Ok(model)
}

/// Generate the next sequential `referencia_interna` for an organization.
///
/// Queries the max existing `referencia_interna` for the org, parses the numeric
/// portion, and increments by 1. Returns `"RI-000001"` if no previous records exist.
async fn generar_siguiente_referencia(
    db: &DatabaseConnection,
    organizacion_id: Uuid,
) -> Result<String, AppError> {
    // Find the latest recibo_informal for this org, ordered by referencia_interna descending
    let ultimo = recibo_informal::Entity::find()
        .filter(recibo_informal::Column::OrganizacionId.eq(organizacion_id))
        .order_by_desc(recibo_informal::Column::ReferenciaInterna)
        .one(db)
        .await?;

    let siguiente_numero = match ultimo {
        Some(recibo) => {
            // Parse "RI-NNNNNN" -> extract NNNNNN and increment
            let num_str = recibo
                .referencia_interna
                .strip_prefix("RI-")
                .ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!(
                        "Formato de referencia_interna inválido: {ref_interna}",
                        ref_interna = recibo.referencia_interna
                    ))
                })?;
            let num: u32 = num_str.parse().map_err(|_| {
                AppError::Internal(anyhow::anyhow!(
                    "No se pudo parsear número de referencia: {num_str}"
                ))
            })?;
            num + 1
        }
        None => 1,
    };

    Ok(format!("RI-{siguiente_numero:06}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    #[test]
    fn formato_referencia_interna() {
        // Verify the format is correct for various numbers
        assert_eq!(format!("RI-{:06}", 1), "RI-000001");
        assert_eq!(format!("RI-{:06}", 42), "RI-000042");
        assert_eq!(format!("RI-{:06}", 999_999), "RI-999999");
    }

    #[test]
    fn parse_referencia_interna() {
        let ref_str = "RI-000015";
        let num_str = ref_str.strip_prefix("RI-").unwrap();
        let num: u32 = num_str.parse().unwrap();
        assert_eq!(num, 15);
    }

    #[test]
    fn parse_referencia_interna_leading_zeros() {
        let ref_str = "RI-000001";
        let num_str = ref_str.strip_prefix("RI-").unwrap();
        let num: u32 = num_str.parse().unwrap();
        assert_eq!(num, 1);
    }
}
