package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class ContratoDto(
    val id: String,
    @SerialName("propiedadId") val propiedadId: String,
    @SerialName("inquilinoId") val inquilinoId: String,
    @SerialName("fechaInicio") val fechaInicio: String,
    @SerialName("fechaFin") val fechaFin: String,
    @SerialName("montoMensual") val montoMensual: String,
    val deposito: String? = null,
    val moneda: String,
    val estado: String,
    @SerialName("createdAt") val createdAt: String,
    @SerialName("updatedAt") val updatedAt: String,
)

@Serializable
data class CreateContratoRequest(
    @SerialName("propiedadId") val propiedadId: String,
    @SerialName("inquilinoId") val inquilinoId: String,
    @SerialName("fechaInicio") val fechaInicio: String,
    @SerialName("fechaFin") val fechaFin: String,
    @SerialName("montoMensual") val montoMensual: String,
    val deposito: String? = null,
    val moneda: String? = null,
)

@Serializable
data class RenovarContratoRequest(
    @SerialName("fechaFin") val fechaFin: String,
    @SerialName("montoMensual") val montoMensual: String,
)

@Serializable
data class TerminarContratoRequest(@SerialName("fechaTerminacion") val fechaTerminacion: String)
