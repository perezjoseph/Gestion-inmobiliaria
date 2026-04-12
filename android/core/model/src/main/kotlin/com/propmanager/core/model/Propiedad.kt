package com.propmanager.core.model

import java.math.BigDecimal
import java.time.Instant

data class Propiedad(
    val id: String,
    val titulo: String,
    val descripcion: String?,
    val direccion: String,
    val ciudad: String,
    val provincia: String,
    val tipoPropiedad: String,
    val habitaciones: Int?,
    val banos: Int?,
    val areaM2: BigDecimal?,
    val precio: BigDecimal,
    val moneda: String,
    val estado: String,
    val imagenes: List<String>,
    val createdAt: Instant,
    val updatedAt: Instant,
    val isPendingSync: Boolean = false
)
