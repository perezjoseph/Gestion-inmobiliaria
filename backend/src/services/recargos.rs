use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use uuid::Uuid;

use crate::entities::{configuracion, contrato, pago};
use crate::errors::AppError;

const CLAVE_RECARGO_DEFECTO: &str = "recargo_porcentaje_defecto";

/// Calcula el recargo: monto * (porcentaje / 100), redondeado a 2 decimales.
/// Función pura, sin I/O.
pub fn calcular_recargo(monto: Decimal, porcentaje: Decimal) -> Decimal {
    (monto * porcentaje / Decimal::from(100)).round_dp(2)
}

/// Resuelve el porcentaje de recargo efectivo: contrato > org > None.
///
/// 1. Si el contrato tiene `recargo_porcentaje` definido, lo retorna.
/// 2. Si no, busca en `configuracion` con clave `recargo_porcentaje_defecto`.
/// 3. Si tampoco existe, retorna `None`.
pub async fn resolver_porcentaje_recargo<C: ConnectionTrait>(
    db: &C,
    contrato: &contrato::Model,
) -> Result<Option<Decimal>, AppError> {
    if let Some(porcentaje) = contrato.recargo_porcentaje {
        return Ok(Some(porcentaje));
    }

    let config = configuracion::Entity::find_by_id(CLAVE_RECARGO_DEFECTO)
        .one(db)
        .await?;

    match config {
        Some(record) => {
            let porcentaje: Option<Decimal> = parse_decimal_from_json(&record.valor)?;
            Ok(porcentaje)
        }
        None => Ok(None),
    }
}

/// Calcula y almacena el recargo en un pago atrasado.
///
/// Resuelve el porcentaje efectivo, calcula el recargo si hay porcentaje,
/// actualiza el campo `pago.recargo`, y retorna el valor calculado.
/// Si el porcentaje es `None`, retorna `Ok(None)` sin actualizar.
pub async fn aplicar_recargo<C: ConnectionTrait>(
    db: &C,
    pago_id: Uuid,
    contrato: &contrato::Model,
) -> Result<Option<Decimal>, AppError> {
    let porcentaje = resolver_porcentaje_recargo(db, contrato).await?;

    let Some(porcentaje) = porcentaje else {
        return Ok(None);
    };

    let pago_record = pago::Entity::find_by_id(pago_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Pago no encontrado".to_string()))?;

    let recargo = calcular_recargo(pago_record.monto, porcentaje);

    let mut active: pago::ActiveModel = pago_record.into();
    active.recargo = Set(Some(recargo));
    active.update(db).await?;

    Ok(Some(recargo))
}

/// Parses a Decimal from a JSONB `valor` field.
///
/// The valor field can contain a JSON number (e.g., `5.5`) or a JSON string
/// (e.g., `"5.50"`). Returns `None` if the value is JSON null.
fn parse_decimal_from_json(valor: &serde_json::Value) -> Result<Option<Decimal>, AppError> {
    if valor.is_null() {
        return Ok(None);
    }

    if let Some(s) = valor.as_str() {
        let d: Decimal = s
            .parse()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error parseando recargo: {e}")))?;
        return Ok(Some(d));
    }

    if let Some(n) = valor.as_f64() {
        let d = Decimal::try_from(n)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Error parseando recargo: {e}")))?;
        return Ok(Some(d.round_dp(2)));
    }

    Err(AppError::Internal(anyhow::anyhow!(
        "Formato inesperado en configuración de recargo"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn calcular_recargo_cinco_porciento() {
        let monto = Decimal::from(1000);
        let porcentaje = Decimal::from(5);
        assert_eq!(calcular_recargo(monto, porcentaje), Decimal::from(50));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn calcular_recargo_diez_porciento_con_decimales() {
        let monto = Decimal::from_str("1500.50").unwrap();
        let porcentaje = Decimal::from(10);
        assert_eq!(
            calcular_recargo(monto, porcentaje),
            Decimal::from_str("150.05").unwrap()
        );
    }

    #[test]
    fn calcular_recargo_cero_porciento() {
        let monto = Decimal::from(5000);
        let porcentaje = Decimal::from(0);
        assert_eq!(
            calcular_recargo(monto, porcentaje),
            Decimal::ZERO
        );
    }

    #[test]
    fn calcular_recargo_cien_porciento() {
        let monto = Decimal::from(2500);
        let porcentaje = Decimal::from(100);
        assert_eq!(calcular_recargo(monto, porcentaje), Decimal::from(2500));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn calcular_recargo_redondeo_dos_decimales() {
        // 333.33 * 3.33% = 11.099889 → rounded to 11.10
        let monto = Decimal::from_str("333.33").unwrap();
        let porcentaje = Decimal::from_str("3.33").unwrap();
        assert_eq!(
            calcular_recargo(monto, porcentaje),
            Decimal::from_str("11.10").unwrap()
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn parse_decimal_from_json_string() {
        let valor = serde_json::json!("5.50");
        let result = parse_decimal_from_json(&valor).unwrap();
        assert_eq!(result, Some(Decimal::from_str("5.50").unwrap()));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn parse_decimal_from_json_number() {
        let valor = serde_json::json!(10.25);
        let result = parse_decimal_from_json(&valor).unwrap();
        assert_eq!(result, Some(Decimal::from_str("10.25").unwrap()));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn parse_decimal_from_json_null() {
        let valor = serde_json::Value::Null;
        let result = parse_decimal_from_json(&valor).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn parse_decimal_from_json_invalid() {
        let valor = serde_json::json!({"nested": "object"});
        let result = parse_decimal_from_json(&valor);
        assert!(result.is_err());
    }
}
