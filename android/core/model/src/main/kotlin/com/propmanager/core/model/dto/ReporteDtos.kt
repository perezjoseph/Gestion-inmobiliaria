package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class IngresoReporte(
    @SerialName("propiedadTitulo") val propiedadTitulo: String,
    @SerialName("inquilinoNombre") val inquilinoNombre: String,
    val monto: String,
    val moneda: String,
    val estado: String
)

@Serializable
data class IngresoReporteSummary(
    val rows: List<IngresoReporte>,
    @SerialName("totalPagado") val totalPagado: String,
    @SerialName("totalPendiente") val totalPendiente: String,
    @SerialName("totalAtrasado") val totalAtrasado: String,
    @SerialName("tasaOcupacion") val tasaOcupacion: Double,
    @SerialName("generatedAt") val generatedAt: String,
    @SerialName("generatedBy") val generatedBy: String
)

@Serializable
data class RentabilidadReporte(
    @SerialName("propiedadId") val propiedadId: String,
    @SerialName("propiedadTitulo") val propiedadTitulo: String,
    @SerialName("totalIngresos") val totalIngresos: String,
    @SerialName("totalGastos") val totalGastos: String,
    @SerialName("ingresoNeto") val ingresoNeto: String,
    val moneda: String
)

@Serializable
data class RentabilidadReporteSummary(
    val rows: List<RentabilidadReporte>,
    @SerialName("totalIngresos") val totalIngresos: String,
    @SerialName("totalGastos") val totalGastos: String,
    @SerialName("totalNeto") val totalNeto: String,
    val mes: Int,
    val anio: Int,
    @SerialName("generatedAt") val generatedAt: String,
    @SerialName("generatedBy") val generatedBy: String
)

@Serializable
data class HistorialPagoReporte(
    @SerialName("contratoId") val contratoId: String,
    val monto: String,
    val moneda: String,
    @SerialName("fechaVencimiento") val fechaVencimiento: String,
    @SerialName("fechaPago") val fechaPago: String? = null,
    val estado: String
)
