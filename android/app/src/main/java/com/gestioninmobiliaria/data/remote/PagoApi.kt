package com.gestioninmobiliaria.data.remote

import com.gestioninmobiliaria.data.model.Pago
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.POST

interface PagoApi {
    @GET("pagos")
    suspend fun getPagos(): List<Pago>

    @POST("pagos")
    suspend fun createPago(@Body request: CreatePagoRequest): Pago
}

@kotlinx.serialization.Serializable
data class CreatePagoRequest(
    val contrato_id: Int,
    val monto: Double,
    val moneda: String,
    val metodo_pago: String,
    val fecha_pago: String,
)
