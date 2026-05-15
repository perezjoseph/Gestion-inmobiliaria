package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class UnidadDto(
    val id: String,
    @SerialName("propiedadId") val propiedadId: String,
    @SerialName("numeroUnidad") val numeroUnidad: String,
    val piso: Int? = null,
    val habitaciones: Int? = null,
    val banos: Int? = null,
    @SerialName("areaM2") val areaM2: String? = null,
    val precio: String,
    val moneda: String,
    val estado: String,
    val descripcion: String? = null,
    @SerialName("gastosCount") val gastosCount: Long? = null,
    @SerialName("mantenimientoCount") val mantenimientoCount: Long? = null,
    @SerialName("createdAt") val createdAt: String,
    @SerialName("updatedAt") val updatedAt: String,
)

@Serializable
data class CreateUnidadRequest(
    @SerialName("numeroUnidad") val numeroUnidad: String,
    val piso: Int? = null,
    val habitaciones: Int? = null,
    val banos: Int? = null,
    @SerialName("areaM2") val areaM2: String? = null,
    val precio: String,
    val moneda: String? = null,
    val estado: String? = null,
    val descripcion: String? = null,
)

@Serializable
data class UpdateUnidadRequest(
    @SerialName("numeroUnidad") val numeroUnidad: String? = null,
    val piso: Int? = null,
    val habitaciones: Int? = null,
    val banos: Int? = null,
    @SerialName("areaM2") val areaM2: String? = null,
    val precio: String? = null,
    val moneda: String? = null,
    val estado: String? = null,
    val descripcion: String? = null,
)
