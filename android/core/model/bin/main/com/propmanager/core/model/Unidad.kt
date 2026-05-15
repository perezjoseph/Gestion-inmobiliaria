package com.propmanager.core.model

import java.math.BigDecimal
import java.time.Instant

data class Unidad(
    val id: String,
    val propiedadId: String,
    val numeroUnidad: String,
    val piso: Int?,
    val habitaciones: Int?,
    val banos: Int?,
    val areaM2: BigDecimal?,
    val precio: BigDecimal,
    val moneda: String,
    val estado: String,
    val descripcion: String?,
    val createdAt: Instant,
    val updatedAt: Instant,
    val isPendingSync: Boolean = false,
)
