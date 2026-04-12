package com.propmanager.core.model

import java.math.BigDecimal
import java.time.Instant
import java.time.LocalDate

data class Pago(
    val id: String,
    val contratoId: String,
    val monto: BigDecimal,
    val moneda: String,
    val fechaPago: LocalDate?,
    val fechaVencimiento: LocalDate,
    val metodoPago: String?,
    val estado: String,
    val notas: String?,
    val createdAt: Instant,
    val updatedAt: Instant,
    val isPendingSync: Boolean = false
)
