package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class DashboardStats(
    @SerialName("totalPropiedades") val totalPropiedades: Long,
    @SerialName("tasaOcupacion") val tasaOcupacion: Double,
    @SerialName("ingresoMensual") val ingresoMensual: String,
    @SerialName("pagosAtrasados") val pagosAtrasados: Long,
    @SerialName("totalGastosMes") val totalGastosMes: String,
)

@Serializable
data class PagoProximo(
    @SerialName("pagoId") val pagoId: String,
    @SerialName("propiedadTitulo") val propiedadTitulo: String,
    @SerialName("inquilinoNombre") val inquilinoNombre: String,
    val monto: String,
    val moneda: String,
    @SerialName("fechaVencimiento") val fechaVencimiento: String,
)

@Serializable
data class ContratoCalendario(
    @SerialName("contratoId") val contratoId: String,
    @SerialName("propiedadTitulo") val propiedadTitulo: String,
    @SerialName("inquilinoNombre") val inquilinoNombre: String,
    @SerialName("fechaFin") val fechaFin: String,
    @SerialName("diasRestantes") val diasRestantes: Long,
    val color: String,
)

@Serializable
data class OcupacionTendencia(
    val mes: Int,
    val anio: Int,
    val tasa: Double,
)

@Serializable
data class IngresosComparacion(
    val esperado: String,
    val cobrado: String,
    val diferencia: String,
)

@Serializable
data class GastosComparacion(
    @SerialName("mesActual") val mesActual: String,
    @SerialName("mesAnterior") val mesAnterior: String,
    @SerialName("porcentajeCambio") val porcentajeCambio: Double,
)
