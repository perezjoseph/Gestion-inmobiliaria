package com.propmanager.core.model

import java.math.BigDecimal
import java.time.Instant
import java.time.LocalDate

data class Gasto(
    val id: String,
    val propiedadId: String,
    val unidadId: String?,
    val categoria: String,
    val descripcion: String,
    val monto: BigDecimal,
    val moneda: String,
    val fechaGasto: LocalDate,
    val estado: String,
    val proveedor: String?,
    val numeroFactura: String?,
    val notas: String?,
    val createdAt: Instant,
    val updatedAt: Instant,
    val isPendingSync: Boolean = false
)
