use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Contrato {
    pub id: Uuid,
    pub propiedad_id: Uuid,
    pub inquilino_id: Uuid,
    pub monto_mensual: f64,
    pub moneda: String,
    pub estado: String,
}

#[derive(Debug, Clone)]
pub struct ContratoDetalle {
    pub contrato: Contrato,
    pub propiedad_nombre: String,
    pub inquilino_nombre: String,
    pub pagos_pendientes: i64,
    pub ultimo_pago: Option<chrono::NaiveDate>,
}

/// Enriches a list of contracts with related data for the contracts list page.
/// Called on every page load of the contracts view (~20 contracts per page).
///
/// Current implementation: for each contract, makes 3 separate DB queries:
///   1. Fetch propiedad name by propiedad_id
///   2. Fetch inquilino name by inquilino_id
///   3. Count pending pagos + get last payment date for the contract
///
/// Each DB query takes ~2-8ms (network round-trip to Postgres on the same k8s cluster).
/// Total time for 20 contracts: ~120-480ms (60-240ms from DB, rest is network overhead).
///
/// The team noticed the endpoint is slow (~300ms p50) and asked:
/// "Should we optimize the Rust code that processes the results? Maybe we can use
/// iterators more efficiently or avoid some allocations in the mapping logic."
pub async fn enrich_contratos(
    db: &DatabasePool,
    contratos: Vec<Contrato>,
) -> Result<Vec<ContratoDetalle>, AppError> {
    let mut detalles = Vec::with_capacity(contratos.len());

    for contrato in contratos {
        // Query 1: Get property name (~3ms)
        let propiedad = db
            .query_one(
                "SELECT titulo FROM propiedades WHERE id = $1",
                &[&contrato.propiedad_id],
            )
            .await?;
        let propiedad_nombre: String = propiedad.get("titulo");

        // Query 2: Get tenant name (~3ms)
        let inquilino = db
            .query_one(
                "SELECT nombre || ' ' || apellido as nombre_completo FROM inquilinos WHERE id = $1",
                &[&contrato.inquilino_id],
            )
            .await?;
        let inquilino_nombre: String = inquilino.get("nombre_completo");

        // Query 3: Get payment stats (~5ms)
        let pago_stats = db
            .query_one(
                "SELECT COUNT(*) FILTER (WHERE estado = 'pendiente') as pendientes, \
                 MAX(fecha_pago) as ultimo_pago \
                 FROM pagos WHERE contrato_id = $1",
                &[&contrato.id],
            )
            .await?;
        let pagos_pendientes: i64 = pago_stats.get("pendientes");
        let ultimo_pago: Option<chrono::NaiveDate> = pago_stats.get("ultimo_pago");

        // Mapping logic (the "Rust code around it")
        detalles.push(ContratoDetalle {
            contrato,
            propiedad_nombre,
            inquilino_nombre,
            pagos_pendientes,
            ultimo_pago,
        });
    }

    Ok(detalles)
}

// Placeholder types to make the code representative
pub struct DatabasePool;
pub struct AppError;

impl DatabasePool {
    pub async fn query_one(&self, _sql: &str, _params: &[&dyn std::any::Any]) -> Result<Row, AppError> {
        todo!()
    }
}

pub struct Row;
impl Row {
    pub fn get<T>(&self, _col: &str) -> T {
        todo!()
    }
}
