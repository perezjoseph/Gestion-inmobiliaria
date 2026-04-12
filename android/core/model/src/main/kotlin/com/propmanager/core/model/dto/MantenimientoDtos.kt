package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class SolicitudDto(
    val id: String,
    @SerialName("propiedadId") val propiedadId: String,
    @SerialName("unidadId") val unidadId: String? = null,
    @SerialName("inquilinoId") val inquilinoId: String? = null,
    val titulo: String,
    val descripcion: String? = null,
    val estado: String,
    val prioridad: String,
    @SerialName("nombreProveedor") val nombreProveedor: String? = null,
    @SerialName("telefonoProveedor") val telefonoProveedor: String? = null,
    @SerialName("emailProveedor") val emailProveedor: String? = null,
    @SerialName("costoMonto") val costoMonto: String? = null,
    @SerialName("costoMoneda") val costoMoneda: String? = null,
    @SerialName("fechaInicio") val fechaInicio: String? = null,
    @SerialName("fechaFin") val fechaFin: String? = null,
    val notas: List<NotaDto>? = null,
    @SerialName("createdAt") val createdAt: String,
    @SerialName("updatedAt") val updatedAt: String,
)

@Serializable
data class NotaDto(
    val id: String,
    @SerialName("solicitudId") val solicitudId: String,
    @SerialName("autorId") val autorId: String,
    val contenido: String,
    @SerialName("createdAt") val createdAt: String,
)

@Serializable
data class CreateSolicitudRequest(
    @SerialName("propiedadId") val propiedadId: String,
    @SerialName("unidadId") val unidadId: String? = null,
    @SerialName("inquilinoId") val inquilinoId: String? = null,
    val titulo: String,
    val descripcion: String? = null,
    val prioridad: String? = null,
    @SerialName("nombreProveedor") val nombreProveedor: String? = null,
    @SerialName("telefonoProveedor") val telefonoProveedor: String? = null,
    @SerialName("emailProveedor") val emailProveedor: String? = null,
    @SerialName("costoMonto") val costoMonto: String? = null,
    @SerialName("costoMoneda") val costoMoneda: String? = null,
)

@Serializable
data class UpdateSolicitudRequest(
    val titulo: String? = null,
    val descripcion: String? = null,
    val prioridad: String? = null,
    @SerialName("nombreProveedor") val nombreProveedor: String? = null,
    @SerialName("telefonoProveedor") val telefonoProveedor: String? = null,
    @SerialName("emailProveedor") val emailProveedor: String? = null,
    @SerialName("costoMonto") val costoMonto: String? = null,
    @SerialName("costoMoneda") val costoMoneda: String? = null,
    @SerialName("unidadId") val unidadId: String? = null,
    @SerialName("inquilinoId") val inquilinoId: String? = null,
)

@Serializable
data class UpdateEstadoRequest(
    val estado: String,
)

@Serializable
data class CreateNotaRequest(
    val contenido: String,
)
