use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::io::Cursor;
use std::str::FromStr;
use uuid::Uuid;

use crate::entities::{inquilino, propiedad};
use crate::errors::AppError;
use crate::models::gasto::CreateGastoRequest;
use crate::models::importacion::{ImportError, ImportFormat, ImportResult};
use crate::services::gastos;

fn parse_csv_rows(data: &[u8]) -> Result<Vec<Vec<String>>, AppError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(data);

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| AppError::Validation(format!("Error leyendo encabezados CSV: {e}")))?
        .iter()
        .map(|h| h.trim().to_lowercase())
        .collect();

    let mut rows = Vec::new();
    rows.push(headers);

    for result in reader.records() {
        let record =
            result.map_err(|e| AppError::Validation(format!("Error leyendo fila CSV: {e}")))?;
        let row: Vec<String> = record.iter().map(|f| f.trim().to_string()).collect();
        rows.push(row);
    }

    Ok(rows)
}

fn parse_xlsx_rows(data: &[u8]) -> Result<Vec<Vec<String>>, AppError> {
    use calamine::{Reader, Xlsx};

    let cursor = Cursor::new(data);
    let mut workbook: Xlsx<_> = Xlsx::new(cursor)
        .map_err(|e| AppError::Validation(format!("Error abriendo archivo XLSX: {e}")))?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| AppError::Validation("El archivo XLSX no contiene hojas".to_string()))?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::Validation(format!("Error leyendo hoja XLSX: {e}")))?;

    let mut rows = Vec::new();
    for (i, row) in range.rows().enumerate() {
        let values: Vec<String> = row
            .iter()
            .map(|cell| {
                let s = cell.to_string();
                if i == 0 {
                    s.trim().to_lowercase()
                } else {
                    s.trim().to_string()
                }
            })
            .collect();
        rows.push(values);
    }

    Ok(rows)
}

fn parse_rows(data: &[u8], formato: ImportFormat) -> Result<Vec<Vec<String>>, AppError> {
    match formato {
        ImportFormat::Csv => parse_csv_rows(data),
        ImportFormat::Xlsx => parse_xlsx_rows(data),
        ImportFormat::Image => Err(AppError::Validation(
            "Las imágenes deben procesarse mediante el pipeline OCR".to_string(),
        )),
    }
}

fn find_column_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h == name)
}

fn get_field(row: &[String], idx: Option<usize>) -> &str {
    idx.and_then(|i| row.get(i))
        .map_or("", String::as_str)
}

fn validate_required_fields<'a>(fields: &[(&str, &'a str)]) -> Vec<&'a str> {
    fields
        .iter()
        .filter_map(|(value, msg)| if value.is_empty() { Some(*msg) } else { None })
        .collect()
}

fn non_empty_to_option(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

struct PropiedadIndices {
    titulo: Option<usize>,
    direccion: Option<usize>,
    ciudad: Option<usize>,
    provincia: Option<usize>,
    tipo_propiedad: Option<usize>,
    precio: Option<usize>,
    moneda: Option<usize>,
    descripcion: Option<usize>,
    estado: Option<usize>,
}

fn process_propiedad_row(
    row: &[String],
    idx: &PropiedadIndices,
) -> Result<propiedad::ActiveModel, String> {
    let titulo = get_field(row, idx.titulo);
    let direccion = get_field(row, idx.direccion);
    let ciudad = get_field(row, idx.ciudad);
    let provincia = get_field(row, idx.provincia);
    let tipo_propiedad = get_field(row, idx.tipo_propiedad);
    let precio_str = get_field(row, idx.precio);

    let errores = validate_required_fields(&[
        (titulo, "titulo es requerido"),
        (direccion, "direccion es requerida"),
        (ciudad, "ciudad es requerida"),
        (provincia, "provincia es requerida"),
        (tipo_propiedad, "tipo_propiedad es requerido"),
        (precio_str, "precio es requerido"),
    ]);

    if !errores.is_empty() {
        return Err(errores.join(", "));
    }

    let precio =
        Decimal::from_str(precio_str).map_err(|_| format!("precio inválido: {precio_str}"))?;

    let moneda = get_field(row, idx.moneda);
    let moneda = if moneda.is_empty() { "DOP" } else { moneda };
    let descripcion = get_field(row, idx.descripcion);
    let estado = get_field(row, idx.estado);
    let estado = if estado.is_empty() {
        "disponible"
    } else {
        estado
    };

    let now = Utc::now().into();
    Ok(propiedad::ActiveModel {
        id: Set(Uuid::new_v4()),
        titulo: Set(titulo.to_string()),
        descripcion: Set(non_empty_to_option(descripcion)),
        direccion: Set(direccion.to_string()),
        ciudad: Set(ciudad.to_string()),
        provincia: Set(provincia.to_string()),
        tipo_propiedad: Set(tipo_propiedad.to_string()),
        habitaciones: Set(None),
        banos: Set(None),
        area_m2: Set(None),
        precio: Set(precio),
        moneda: Set(moneda.to_string()),
        estado: Set(estado.to_string()),
        imagenes: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    })
}

pub async fn importar_propiedades(
    db: &DatabaseConnection,
    data: &[u8],
    formato: ImportFormat,
) -> Result<ImportResult, AppError> {
    let rows = parse_rows(data, formato)?;
    if rows.is_empty() {
        return Ok(ImportResult {
            total_filas: 0,
            exitosos: 0,
            fallidos: Vec::new(),
        });
    }

    let headers = &rows[0];
    let idx = PropiedadIndices {
        titulo: find_column_index(headers, "titulo"),
        direccion: find_column_index(headers, "direccion"),
        ciudad: find_column_index(headers, "ciudad"),
        provincia: find_column_index(headers, "provincia"),
        tipo_propiedad: find_column_index(headers, "tipo_propiedad"),
        precio: find_column_index(headers, "precio"),
        moneda: find_column_index(headers, "moneda"),
        descripcion: find_column_index(headers, "descripcion"),
        estado: find_column_index(headers, "estado"),
    };

    let data_rows = &rows[1..];
    let total_filas = data_rows.len();
    let mut exitosos = 0usize;
    let mut fallidos = Vec::new();

    for (i, row) in data_rows.iter().enumerate() {
        let fila = i + 2;
        let model = match process_propiedad_row(row, &idx) {
            Ok(m) => m,
            Err(error) => {
                fallidos.push(ImportError { fila, error });
                continue;
            }
        };

        match model.insert(db).await {
            Ok(_) => exitosos += 1,
            Err(e) => {
                fallidos.push(ImportError {
                    fila,
                    error: format!("Error insertando: {e}"),
                });
            }
        }
    }

    Ok(ImportResult {
        total_filas,
        exitosos,
        fallidos,
    })
}

fn process_inquilino_row(
    row: &[String],
    idx_nombre: Option<usize>,
    idx_apellido: Option<usize>,
    idx_cedula: Option<usize>,
    idx_email: Option<usize>,
    idx_telefono: Option<usize>,
) -> Result<(&str, inquilino::ActiveModel), String> {
    let nombre = get_field(row, idx_nombre);
    let apellido = get_field(row, idx_apellido);
    let cedula = get_field(row, idx_cedula);

    let errores = validate_required_fields(&[
        (nombre, "nombre es requerido"),
        (apellido, "apellido es requerido"),
        (cedula, "cedula es requerida"),
    ]);

    if !errores.is_empty() {
        return Err(errores.join(", "));
    }

    let email = get_field(row, idx_email);
    let telefono = get_field(row, idx_telefono);

    let now = Utc::now().into();
    let model = inquilino::ActiveModel {
        id: Set(Uuid::new_v4()),
        nombre: Set(nombre.to_string()),
        apellido: Set(apellido.to_string()),
        email: Set(non_empty_to_option(email)),
        telefono: Set(non_empty_to_option(telefono)),
        cedula: Set(cedula.to_string()),
        contacto_emergencia: Set(None),
        notas: Set(None),
        documentos: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    Ok((cedula, model))
}

pub async fn importar_inquilinos(
    db: &DatabaseConnection,
    data: &[u8],
    formato: ImportFormat,
) -> Result<ImportResult, AppError> {
    let rows = parse_rows(data, formato)?;
    if rows.is_empty() {
        return Ok(ImportResult {
            total_filas: 0,
            exitosos: 0,
            fallidos: Vec::new(),
        });
    }

    let headers = &rows[0];
    let idx_nombre = find_column_index(headers, "nombre");
    let idx_apellido = find_column_index(headers, "apellido");
    let idx_cedula = find_column_index(headers, "cedula");
    let idx_email = find_column_index(headers, "email");
    let idx_telefono = find_column_index(headers, "telefono");

    let data_rows = &rows[1..];
    let total_filas = data_rows.len();
    let mut exitosos = 0usize;
    let mut fallidos = Vec::new();

    for (i, row) in data_rows.iter().enumerate() {
        let fila = i + 2;
        let (cedula, model) = match process_inquilino_row(
            row,
            idx_nombre,
            idx_apellido,
            idx_cedula,
            idx_email,
            idx_telefono,
        ) {
            Ok(result) => result,
            Err(error) => {
                fallidos.push(ImportError { fila, error });
                continue;
            }
        };

        let existing = inquilino::Entity::find()
            .filter(inquilino::Column::Cedula.eq(cedula))
            .one(db)
            .await?;

        if existing.is_some() {
            fallidos.push(ImportError {
                fila,
                error: format!("Cédula duplicada: {cedula}"),
            });
            continue;
        }

        match model.insert(db).await {
            Ok(_) => exitosos += 1,
            Err(e) => {
                fallidos.push(ImportError {
                    fila,
                    error: format!("Error insertando: {e}"),
                });
            }
        }
    }

    Ok(ImportResult {
        total_filas,
        exitosos,
        fallidos,
    })
}

struct GastoIndices {
    propiedad_id: Option<usize>,
    categoria: Option<usize>,
    descripcion: Option<usize>,
    monto: Option<usize>,
    moneda: Option<usize>,
    fecha_gasto: Option<usize>,
    unidad_id: Option<usize>,
    proveedor: Option<usize>,
    numero_factura: Option<usize>,
    notas: Option<usize>,
}

fn process_gasto_row(row: &[String], idx: &GastoIndices) -> Result<CreateGastoRequest, String> {
    let propiedad_id_str = get_field(row, idx.propiedad_id);
    let categoria = get_field(row, idx.categoria);
    let descripcion = get_field(row, idx.descripcion);
    let monto_str = get_field(row, idx.monto);
    let moneda = get_field(row, idx.moneda);
    let fecha_gasto_str = get_field(row, idx.fecha_gasto);

    let errores = validate_required_fields(&[
        (propiedad_id_str, "propiedad_id es requerido"),
        (categoria, "categoria es requerida"),
        (descripcion, "descripcion es requerida"),
        (monto_str, "monto es requerido"),
        (moneda, "moneda es requerida"),
        (fecha_gasto_str, "fecha_gasto es requerida"),
    ]);

    if !errores.is_empty() {
        return Err(errores.join(", "));
    }

    let propiedad_id = Uuid::from_str(propiedad_id_str)
        .map_err(|_| format!("propiedad_id inválido: {propiedad_id_str}"))?;

    let monto = Decimal::from_str(monto_str).map_err(|_| format!("monto inválido: {monto_str}"))?;

    let fecha_gasto = NaiveDate::parse_from_str(fecha_gasto_str, "%Y-%m-%d")
        .map_err(|_| format!("fecha_gasto inválida: {fecha_gasto_str}"))?;

    let unidad_id = parse_optional_uuid(get_field(row, idx.unidad_id))?;

    Ok(CreateGastoRequest {
        propiedad_id,
        unidad_id,
        categoria: categoria.to_string(),
        descripcion: descripcion.to_string(),
        monto,
        moneda: moneda.to_string(),
        fecha_gasto,
        proveedor: non_empty_to_option(get_field(row, idx.proveedor)),
        numero_factura: non_empty_to_option(get_field(row, idx.numero_factura)),
        notas: non_empty_to_option(get_field(row, idx.notas)),
    })
}

fn parse_optional_uuid(value: &str) -> Result<Option<Uuid>, String> {
    if value.is_empty() {
        return Ok(None);
    }
    Uuid::from_str(value)
        .map(Some)
        .map_err(|_| format!("unidad_id inválido: {value}"))
}

pub async fn importar_gastos(
    db: &DatabaseConnection,
    data: &[u8],
    formato: ImportFormat,
    usuario_id: Uuid,
) -> Result<ImportResult, AppError> {
    let rows = parse_rows(data, formato)?;
    if rows.is_empty() {
        return Ok(ImportResult {
            total_filas: 0,
            exitosos: 0,
            fallidos: Vec::new(),
        });
    }

    let headers = &rows[0];
    let idx = GastoIndices {
        propiedad_id: find_column_index(headers, "propiedad_id"),
        categoria: find_column_index(headers, "categoria"),
        descripcion: find_column_index(headers, "descripcion"),
        monto: find_column_index(headers, "monto"),
        moneda: find_column_index(headers, "moneda"),
        fecha_gasto: find_column_index(headers, "fecha_gasto"),
        unidad_id: find_column_index(headers, "unidad_id"),
        proveedor: find_column_index(headers, "proveedor"),
        numero_factura: find_column_index(headers, "numero_factura"),
        notas: find_column_index(headers, "notas"),
    };

    let data_rows = &rows[1..];
    if data_rows.is_empty() {
        return Err(AppError::Validation(
            "El archivo CSV está vacío o no contiene filas válidas".to_string(),
        ));
    }
    let total_filas = data_rows.len();
    let mut exitosos = 0usize;
    let mut fallidos = Vec::new();

    for (i, row) in data_rows.iter().enumerate() {
        let fila = i + 2;
        let request = match process_gasto_row(row, &idx) {
            Ok(r) => r,
            Err(error) => {
                fallidos.push(ImportError { fila, error });
                continue;
            }
        };

        match gastos::create(db, request, usuario_id).await {
            Ok(_) => exitosos += 1,
            Err(e) => {
                fallidos.push(ImportError {
                    fila,
                    error: format!("{e}"),
                });
            }
        }
    }

    if exitosos == 0 && !fallidos.is_empty() {
        return Err(AppError::Validation(
            "El archivo CSV está vacío o no contiene filas válidas".to_string(),
        ));
    }

    Ok(ImportResult {
        total_filas,
        exitosos,
        fallidos,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_csv_rows_parses_headers_and_data() {
        let csv_data = b"titulo,direccion,ciudad,provincia,tipo_propiedad,precio\nCasa 1,Calle 1,Santo Domingo,Distrito Nacional,casa,50000\n";
        let rows = parse_csv_rows(csv_data).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0], "titulo");
        assert_eq!(rows[1][0], "Casa 1");
        assert_eq!(rows[1][5], "50000");
    }

    #[test]
    fn parse_csv_rows_trims_whitespace() {
        let csv_data = b"nombre , apellido , cedula \n Juan , Perez , 001-1234567-8 \n";
        let rows = parse_csv_rows(csv_data).unwrap();
        assert_eq!(rows[0][0], "nombre");
        assert_eq!(rows[1][0], "Juan");
        assert_eq!(rows[1][2], "001-1234567-8");
    }

    #[test]
    fn parse_csv_rows_empty_file_returns_headers_only() {
        let csv_data = b"nombre,apellido,cedula\n";
        let rows = parse_csv_rows(csv_data).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn find_column_index_finds_existing_column() {
        let headers = vec![
            "titulo".to_string(),
            "direccion".to_string(),
            "ciudad".to_string(),
        ];
        assert_eq!(find_column_index(&headers, "direccion"), Some(1));
    }

    #[test]
    fn find_column_index_returns_none_for_missing() {
        let headers = vec!["titulo".to_string(), "direccion".to_string()];
        assert_eq!(find_column_index(&headers, "ciudad"), None);
    }

    #[test]
    fn get_field_returns_value_at_index() {
        let row = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(get_field(&row, Some(1)), "b");
    }

    #[test]
    fn get_field_returns_empty_for_none_index() {
        let row = vec!["a".to_string()];
        assert_eq!(get_field(&row, None), "");
    }

    #[test]
    fn get_field_returns_empty_for_out_of_bounds() {
        let row = vec!["a".to_string()];
        assert_eq!(get_field(&row, Some(5)), "");
    }

    #[test]
    fn validate_propiedad_required_fields() {
        let required = [
            "titulo",
            "direccion",
            "ciudad",
            "provincia",
            "tipo_propiedad",
            "precio",
        ];
        let values = ["", "Calle 1", "SD", "DN", "casa", "50000"];
        let mut errores = Vec::new();
        for (field, val) in required.iter().zip(values.iter()) {
            if val.is_empty() {
                errores.push(format!("{field} es requerido"));
            }
        }
        assert_eq!(errores.len(), 1);
        assert!(errores[0].contains("titulo"));
    }

    #[test]
    fn validate_inquilino_required_fields() {
        let required = ["nombre", "apellido", "cedula"];
        let values = ["Juan", "", "001-1234567-8"];
        let mut errores = Vec::new();
        for (field, val) in required.iter().zip(values.iter()) {
            if val.is_empty() {
                errores.push(format!("{field} es requerido"));
            }
        }
        assert_eq!(errores.len(), 1);
        assert!(errores[0].contains("apellido"));
    }

    #[test]
    fn precio_parsing_valid() {
        let result = Decimal::from_str("50000.50");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Decimal::new(5_000_050, 2));
    }

    #[test]
    fn precio_parsing_invalid() {
        let result = Decimal::from_str("not_a_number");
        assert!(result.is_err());
    }

    #[test]
    fn import_format_equality() {
        assert_eq!(ImportFormat::Csv, ImportFormat::Csv);
        assert_eq!(ImportFormat::Xlsx, ImportFormat::Xlsx);
        assert_ne!(ImportFormat::Csv, ImportFormat::Xlsx);
    }

    #[test]
    fn parse_csv_rows_gastos_columns() {
        let csv_data = b"propiedad_id,categoria,descripcion,monto,moneda,fecha_gasto,proveedor,numero_factura,notas\n550e8400-e29b-41d4-a716-446655440000,mantenimiento,Reparacion techo,15000.50,DOP,2025-04-01,Constructora ABC,FAC-001,Urgente\n";
        let rows = parse_csv_rows(csv_data).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0], "propiedad_id");
        assert_eq!(rows[0][1], "categoria");
        assert_eq!(rows[0][2], "descripcion");
        assert_eq!(rows[0][3], "monto");
        assert_eq!(rows[0][4], "moneda");
        assert_eq!(rows[0][5], "fecha_gasto");
        assert_eq!(rows[0][6], "proveedor");
        assert_eq!(rows[0][7], "numero_factura");
        assert_eq!(rows[0][8], "notas");
        assert_eq!(rows[1][0], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(rows[1][1], "mantenimiento");
        assert_eq!(rows[1][3], "15000.50");
        assert_eq!(rows[1][5], "2025-04-01");
        assert_eq!(rows[1][6], "Constructora ABC");
    }

    #[test]
    fn gastos_column_index_lookup() {
        let headers: Vec<String> = vec![
            "propiedad_id",
            "categoria",
            "descripcion",
            "monto",
            "moneda",
            "fecha_gasto",
            "unidad_id",
            "proveedor",
            "numero_factura",
            "notas",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        assert_eq!(find_column_index(&headers, "propiedad_id"), Some(0));
        assert_eq!(find_column_index(&headers, "categoria"), Some(1));
        assert_eq!(find_column_index(&headers, "descripcion"), Some(2));
        assert_eq!(find_column_index(&headers, "monto"), Some(3));
        assert_eq!(find_column_index(&headers, "moneda"), Some(4));
        assert_eq!(find_column_index(&headers, "fecha_gasto"), Some(5));
        assert_eq!(find_column_index(&headers, "unidad_id"), Some(6));
        assert_eq!(find_column_index(&headers, "proveedor"), Some(7));
        assert_eq!(find_column_index(&headers, "numero_factura"), Some(8));
        assert_eq!(find_column_index(&headers, "notas"), Some(9));
    }

    #[test]
    fn gastos_required_field_validation_all_missing() {
        let required = [
            "propiedad_id",
            "categoria",
            "descripcion",
            "monto",
            "moneda",
            "fecha_gasto",
        ];
        let values = ["", "", "", "", "", ""];
        let mut errores = Vec::new();
        for (field, val) in required.iter().zip(values.iter()) {
            if val.is_empty() {
                errores.push(format!("{field} es requerido"));
            }
        }
        assert_eq!(errores.len(), 6);
    }

    #[test]
    fn gastos_required_field_validation_partial_missing() {
        let required = [
            "propiedad_id",
            "categoria",
            "descripcion",
            "monto",
            "moneda",
            "fecha_gasto",
        ];
        let values = [
            "550e8400-e29b-41d4-a716-446655440000",
            "mantenimiento",
            "",
            "15000",
            "",
            "2025-04-01",
        ];
        let mut errores = Vec::new();
        for (field, val) in required.iter().zip(values.iter()) {
            if val.is_empty() {
                errores.push(format!("{field} es requerido"));
            }
        }
        assert_eq!(errores.len(), 2);
        assert!(errores[0].contains("descripcion"));
        assert!(errores[1].contains("moneda"));
    }

    #[test]
    fn gastos_monto_parsing_valid() {
        let result = Decimal::from_str("15000.50");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Decimal::new(1_500_050, 2));
    }

    #[test]
    fn gastos_monto_parsing_invalid() {
        let result = Decimal::from_str("abc");
        assert!(result.is_err());
    }

    #[test]
    fn gastos_fecha_parsing_valid() {
        let result = NaiveDate::parse_from_str("2025-04-01", "%Y-%m-%d");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap()
        );
    }

    #[test]
    fn gastos_fecha_parsing_invalid() {
        let result = NaiveDate::parse_from_str("01/04/2025", "%Y-%m-%d");
        assert!(result.is_err());
    }

    #[test]
    fn gastos_uuid_parsing_valid() {
        let result = Uuid::from_str("550e8400-e29b-41d4-a716-446655440000");
        assert!(result.is_ok());
    }

    #[test]
    fn gastos_uuid_parsing_invalid() {
        let result = Uuid::from_str("not-a-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn gastos_optional_columns_missing_from_headers() {
        let headers: Vec<String> = vec![
            "propiedad_id",
            "categoria",
            "descripcion",
            "monto",
            "moneda",
            "fecha_gasto",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        assert!(find_column_index(&headers, "unidad_id").is_none());
        assert!(find_column_index(&headers, "proveedor").is_none());
        assert!(find_column_index(&headers, "numero_factura").is_none());
        assert!(find_column_index(&headers, "notas").is_none());
    }

    #[test]
    fn gastos_csv_with_optional_columns_only() {
        let csv_data = b"propiedad_id,categoria,descripcion,monto,moneda,fecha_gasto,unidad_id,proveedor\n550e8400-e29b-41d4-a716-446655440000,impuestos,Impuesto predial,5000,DOP,2025-03-15,,Ayuntamiento\n";
        let rows = parse_csv_rows(csv_data).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(get_field(&rows[1], Some(6)), "");
        assert_eq!(get_field(&rows[1], Some(7)), "Ayuntamiento");
    }
}
