package com.propmanager.core.model

import java.math.BigDecimal
import java.time.Instant
import java.time.LocalDate

data class Contrato(
    val id: String,
    val propiedadId: String,
    val inquilinoId: String,
    val fechaInicio: LocalDate,
    val fechaFin: LocalDate,
    val montoMensual: BigDecimal,
    val deposito: BigDecimal?,
    val moneda: String,
    val estado: String,
    val createdAt: Instant,
    val updatedAt: Instant,
    val isPendingSync: Boolean = false
)
