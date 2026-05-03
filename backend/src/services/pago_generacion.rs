use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

use crate::errors::AppError;

/// Datos de un pago a generar (sin ID ni timestamps).
#[derive(Debug, Clone)]
pub struct PagoGenerado {
    pub monto: Decimal,
    pub moneda: String,
    pub fecha_vencimiento: NaiveDate,
}

/// Calcula los pagos mensuales para un período contractual.
/// Función pura: no accede a la base de datos.
///
/// Itera mes a mes desde el mes de `fecha_inicio` hasta el mes de `fecha_fin` (inclusive).
/// Para cada mes, calcula `fecha_vencimiento` como `(año, mes, min(dia_vencimiento, último_día_del_mes))`.
pub fn calcular_pagos(
    fecha_inicio: NaiveDate,
    fecha_fin: NaiveDate,
    monto_mensual: Decimal,
    moneda: &str,
    dia_vencimiento: u32,
) -> Vec<PagoGenerado> {
    if fecha_inicio > fecha_fin {
        return Vec::new();
    }

    let mut pagos = Vec::new();
    let mut year = fecha_inicio.year();
    let mut month = fecha_inicio.month();

    let end_year = fecha_fin.year();
    let end_month = fecha_fin.month();

    loop {
        let last_day = last_day_of_month(year, month);
        let day = dia_vencimiento.min(last_day);

        // Safe: day is clamped to valid range for this year/month
        if let Some(fecha) = NaiveDate::from_ymd_opt(year, month, day) {
            pagos.push(PagoGenerado {
                monto: monto_mensual,
                moneda: moneda.to_string(),
                fecha_vencimiento: fecha,
            });
        }

        if year == end_year && month == end_month {
            break;
        }

        // Advance to next month
        if month == 12 {
            month = 1;
            year += 1;
        } else {
            month += 1;
        }
    }

    pagos
}

/// Filtra pagos ya existentes comparando por (año, mes) de `fecha_vencimiento`.
pub fn filtrar_existentes(
    pagos_calculados: &[PagoGenerado],
    fechas_existentes: &[NaiveDate],
) -> Vec<PagoGenerado> {
    pagos_calculados
        .iter()
        .filter(|pago| {
            let y = pago.fecha_vencimiento.year();
            let m = pago.fecha_vencimiento.month();
            !fechas_existentes
                .iter()
                .any(|f| f.year() == y && f.month() == m)
        })
        .cloned()
        .collect()
}

/// Valida que `dia_vencimiento` esté entre 1 y 31.
pub fn validar_dia_vencimiento(dia: u32) -> Result<(), AppError> {
    if !(1..=31).contains(&dia) {
        return Err(AppError::Validation(
            "El día de vencimiento debe estar entre 1 y 31".to_string(),
        ));
    }
    Ok(())
}

/// Retorna el último día del mes para un año y mes dados.
fn last_day_of_month(year: i32, month: u32) -> u32 {
    // The first day of the next month, minus one day, gives the last day of this month.
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map_or(28, |d| d.day()) // fallback, should never happen for valid inputs
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::str::FromStr;

    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    // ── calcular_pagos ──────────────────────────────────

    #[test]
    fn calcular_pagos_single_month_returns_one() {
        let inicio = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
        let fin = NaiveDate::from_ymd_opt(2025, 3, 31).unwrap();
        let monto = Decimal::from_str("15000.00").unwrap();

        let pagos = calcular_pagos(inicio, fin, monto, "DOP", 1);

        assert_eq!(pagos.len(), 1);
        assert_eq!(pagos[0].monto, monto);
        assert_eq!(pagos[0].moneda, "DOP");
        assert_eq!(pagos[0].fecha_vencimiento, NaiveDate::from_ymd_opt(2025, 3, 1).unwrap());
    }

    #[test]
    fn calcular_pagos_twelve_months_returns_twelve() {
        let inicio = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let fin = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let monto = Decimal::from_str("15000.00").unwrap();

        let pagos = calcular_pagos(inicio, fin, monto, "USD", 1);

        assert_eq!(pagos.len(), 12);
        for (i, pago) in pagos.iter().enumerate() {
            assert_eq!(pago.monto, monto);
            assert_eq!(pago.moneda, "USD");
            let expected_month = u32::try_from(i + 1).unwrap();
            assert_eq!(pago.fecha_vencimiento.month(), expected_month);
        }
    }

    #[test]
    fn calcular_pagos_dia_31_february_clamps_to_28() {
        let inicio = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let fin = NaiveDate::from_ymd_opt(2025, 2, 28).unwrap();
        let monto = Decimal::from_str("15000.00").unwrap();

        let pagos = calcular_pagos(inicio, fin, monto, "DOP", 31);

        assert_eq!(pagos.len(), 1);
        assert_eq!(
            pagos[0].fecha_vencimiento,
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        );
    }

    #[test]
    fn calcular_pagos_dia_31_february_leap_year_clamps_to_29() {
        let inicio = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
        let fin = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        let monto = Decimal::from_str("15000.00").unwrap();

        let pagos = calcular_pagos(inicio, fin, monto, "DOP", 31);

        assert_eq!(pagos.len(), 1);
        assert_eq!(
            pagos[0].fecha_vencimiento,
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
    }

    #[test]
    fn calcular_pagos_same_day_returns_one() {
        let fecha = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let monto = Decimal::from_str("15000.00").unwrap();

        let pagos = calcular_pagos(fecha, fecha, monto, "DOP", 1);

        assert_eq!(pagos.len(), 1);
        assert_eq!(
            pagos[0].fecha_vencimiento,
            NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()
        );
    }

    // ── filtrar_existentes ──────────────────────────────

    #[test]
    fn filtrar_existentes_all_existing_returns_empty() {
        let monto = Decimal::from_str("15000.00").unwrap();
        let pagos = vec![
            PagoGenerado {
                monto,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            },
            PagoGenerado {
                monto,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
        ];
        let existentes = vec![
            NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 20).unwrap(),
        ];

        let resultado = filtrar_existentes(&pagos, &existentes);

        assert!(resultado.is_empty());
    }

    #[test]
    fn filtrar_existentes_none_existing_returns_all() {
        let monto = Decimal::from_str("15000.00").unwrap();
        let pagos = vec![
            PagoGenerado {
                monto,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            },
            PagoGenerado {
                monto,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            },
        ];
        let existentes: Vec<NaiveDate> = vec![];

        let resultado = filtrar_existentes(&pagos, &existentes);

        assert_eq!(resultado.len(), 2);
    }

    #[test]
    fn filtrar_existentes_some_existing_returns_missing() {
        let monto = Decimal::from_str("15000.00").unwrap();
        let pagos = vec![
            PagoGenerado {
                monto,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            },
            PagoGenerado {
                monto,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            PagoGenerado {
                monto,
                moneda: "DOP".to_string(),
                fecha_vencimiento: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            },
        ];
        // Only January exists
        let existentes = vec![NaiveDate::from_ymd_opt(2025, 1, 10).unwrap()];

        let resultado = filtrar_existentes(&pagos, &existentes);

        assert_eq!(resultado.len(), 2);
        assert_eq!(resultado[0].fecha_vencimiento.month(), 2);
        assert_eq!(resultado[1].fecha_vencimiento.month(), 3);
    }

    // ── validar_dia_vencimiento ─────────────────────────

    #[test]
    fn validar_dia_vencimiento_zero_returns_error() {
        let result = validar_dia_vencimiento(0);
        assert!(result.is_err());
    }

    #[test]
    fn validar_dia_vencimiento_32_returns_error() {
        let result = validar_dia_vencimiento(32);
        assert!(result.is_err());
    }

    #[test]
    fn validar_dia_vencimiento_1_returns_ok() {
        let result = validar_dia_vencimiento(1);
        assert!(result.is_ok());
    }

    #[test]
    fn validar_dia_vencimiento_31_returns_ok() {
        let result = validar_dia_vencimiento(31);
        assert!(result.is_ok());
    }
}
