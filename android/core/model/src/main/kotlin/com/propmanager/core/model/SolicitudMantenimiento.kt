package com.propmanager.core.model

import java.math.BigDecimal
import java.time.Instant

data class SolicitudMantenimiento(
    val id: String,
    val propiedadId: String,
    val unidadId: String?,
    val inquilinoId: String?,
    val titulo: String,
    val descripcion: String?,
    val estado: String,
    val prioridad: String,
    val nombreProveedor: String?,
    val telefonoProveedor: String?,
    val emailProveedor: String?,
    val costoMonto: BigDecimal?,
    val costoMoneda: String?,
    val fechaInicio: Instant?,
    val fechaFin: Instant?,
    val createdAt: Instant,
    val updatedAt: Instant,
    val isPendingSync: Boolean = false
)
