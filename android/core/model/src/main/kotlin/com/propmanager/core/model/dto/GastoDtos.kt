package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class GastoDto(
    val id: String,
    @SerialName("propiedadId") val propiedadId: String,
    @SerialName("unidadId") val unidadId: String? = null,
    val categoria: String,
    val descripcion: String,
    val monto: String,
    val moneda: String,
    @SerialName("fechaGasto") val fechaGasto: String,
    val estado: String,
    val proveedor: String? = null,
    @SerialName("numeroFactura") val numeroFactura: String? = null,
    val notas: String? = null,
    @SerialName("createdAt") val createdAt: String,
    @SerialName("updatedAt") val updatedAt: String
)

@Serializable
data class CreateGastoRequest(
    @SerialName("propiedadId") val propiedadId: String,
    @SerialName("unidadId") val unidadId: String? = null,
    val categoria: String,
    val descripcion: String,
    val monto: String,
    val moneda: String,
    @SerialName("fechaGasto") val fechaGasto: String,
    val proveedor: String? = null,
    @SerialName("numeroFactura") val numeroFactura: String? = null,
    val notas: String? = null
)

@Serializable
data class UpdateGastoRequest(
    val categoria: String? = null,
    val descripcion: String? = null,
    val monto: String? = null,
    val moneda: String? = null,
    @SerialName("fechaGasto") val fechaGasto: String? = null,
    @SerialName("unidadId") val unidadId: String? = null,
    val proveedor: String? = null,
    @SerialName("numeroFactura") val numeroFactura: String? = null,
    val estado: String? = null,
    val notas: String? = null
)
