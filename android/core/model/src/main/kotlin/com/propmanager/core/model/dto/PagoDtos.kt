package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class PagoDto(
    val id: String,
    @SerialName("contratoId") val contratoId: String,
    val monto: String,
    val moneda: String,
    @SerialName("fechaPago") val fechaPago: String? = null,
    @SerialName("fechaVencimiento") val fechaVencimiento: String,
    @SerialName("metodoPago") val metodoPago: String? = null,
    val estado: String,
    val notas: String? = null,
    @SerialName("createdAt") val createdAt: String,
    @SerialName("updatedAt") val updatedAt: String,
)

@Serializable
data class CreatePagoRequest(
    @SerialName("contratoId") val contratoId: String,
    val monto: String,
    val moneda: String? = null,
    @SerialName("fechaPago") val fechaPago: String? = null,
    @SerialName("fechaVencimiento") val fechaVencimiento: String,
    @SerialName("metodoPago") val metodoPago: String? = null,
    val notas: String? = null,
)

@Serializable
data class UpdatePagoRequest(
    val monto: String? = null,
    @SerialName("fechaPago") val fechaPago: String? = null,
    @SerialName("metodoPago") val metodoPago: String? = null,
    val estado: String? = null,
    val notas: String? = null,
)
