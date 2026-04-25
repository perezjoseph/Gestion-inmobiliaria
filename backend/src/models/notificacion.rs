use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagoVencido {
    pub pago_id: Uuid,
    pub propiedad_titulo: String,
    pub inquilino_nombre: String,
    pub inquilino_apellido: String,
    pub monto: Decimal,
    pub moneda: String,
    pub dias_vencido: i64,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn pago_vencido_serializes_to_camel_case() {
        let pv = PagoVencido {
            pago_id: Uuid::new_v4(),
            propiedad_titulo: "Apartamento Centro".into(),
            inquilino_nombre: "Juan".into(),
            inquilino_apellido: "Perez".into(),
            monto: Decimal::new(25000, 0),
            moneda: "DOP".into(),
            dias_vencido: 15,
        };
        let json = serde_json::to_value(&pv).unwrap();
        assert!(json.get("pagoId").is_some());
        assert!(json.get("propiedadTitulo").is_some());
        assert!(json.get("inquilinoNombre").is_some());
        assert!(json.get("inquilinoApellido").is_some());
        assert!(json.get("diasVencido").is_some());
        assert_eq!(json["diasVencido"], 15);
    }

    #[test]
    fn pago_vencido_serializes_zero_days_overdue() {
        let pv = PagoVencido {
            pago_id: Uuid::new_v4(),
            propiedad_titulo: "Local Comercial".into(),
            inquilino_nombre: "Maria".into(),
            inquilino_apellido: "Lopez".into(),
            monto: Decimal::new(5000, 2),
            moneda: "USD".into(),
            dias_vencido: 0,
        };
        let json = serde_json::to_value(&pv).unwrap();
        assert_eq!(json["diasVencido"], 0);
        assert_eq!(json["monto"], "50.00");
        assert_eq!(json["moneda"], "USD");
    }

    #[test]
    fn pago_vencido_serializes_large_days_overdue() {
        let pv = PagoVencido {
            pago_id: Uuid::new_v4(),
            propiedad_titulo: "Casa Playa".into(),
            inquilino_nombre: "Carlos".into(),
            inquilino_apellido: "Ramirez".into(),
            monto: Decimal::new(150_000, 0),
            moneda: "DOP".into(),
            dias_vencido: 365,
        };
        let json = serde_json::to_value(&pv).unwrap();
        assert_eq!(json["diasVencido"], 365);
        assert_eq!(json["propiedadTitulo"], "Casa Playa");
        assert_eq!(json["inquilinoNombre"], "Carlos");
        assert_eq!(json["inquilinoApellido"], "Ramirez");
    }
}
