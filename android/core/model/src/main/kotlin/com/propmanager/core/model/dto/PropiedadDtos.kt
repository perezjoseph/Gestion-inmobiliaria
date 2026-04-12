package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

@Serializable
data class PropiedadDto(
    val id: String,
    val titulo: String,
    val descripcion: String? = null,
    val direccion: String,
    val ciudad: String,
    val provincia: String,
    @SerialName("tipoPropiedad") val tipoPropiedad: String,
    val habitaciones: Int? = null,
    val banos: Int? = null,
    @SerialName("areaM2") val areaM2: String? = null,
    val precio: String,
    val moneda: String,
    val estado: String,
    val imagenes: JsonElement? = null,
    @SerialName("createdAt") val createdAt: String,
    @SerialName("updatedAt") val updatedAt: String,
)

@Serializable
data class CreatePropiedadRequest(
    val titulo: String,
    val descripcion: String? = null,
    val direccion: String,
    val ciudad: String,
    val provincia: String,
    @SerialName("tipoPropiedad") val tipoPropiedad: String,
    val habitaciones: Int? = null,
    val banos: Int? = null,
    @SerialName("areaM2") val areaM2: String? = null,
    val precio: String,
    val moneda: String? = null,
    val estado: String? = null,
    val imagenes: JsonElement? = null,
)

@Serializable
data class UpdatePropiedadRequest(
    val titulo: String? = null,
    val descripcion: String? = null,
    val direccion: String? = null,
    val ciudad: String? = null,
    val provincia: String? = null,
    @SerialName("tipoPropiedad") val tipoPropiedad: String? = null,
    val habitaciones: Int? = null,
    val banos: Int? = null,
    @SerialName("areaM2") val areaM2: String? = null,
    val precio: String? = null,
    val moneda: String? = null,
    val estado: String? = null,
    val imagenes: JsonElement? = null,
)
