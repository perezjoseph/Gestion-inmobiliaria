package com.propmanager.core.network.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import retrofit2.Response
import retrofit2.http.GET

@Serializable
data class PagoVencidoDto(
    @SerialName("pagoId") val pagoId: String,
    @SerialName("propiedadTitulo") val propiedadTitulo: String,
    @SerialName("inquilinoNombre") val inquilinoNombre: String,
    @SerialName("inquilinoApellido") val inquilinoApellido: String,
    val monto: String,
    val moneda: String,
    @SerialName("diasVencido") val diasVencido: Long,
)

@Suppress("kotlin:S6517")
interface NotificacionesApiService {
    @GET("api/notificaciones/pagos-vencidos")
    suspend fun pagosVencidos(): Response<List<PagoVencidoDto>>
}
