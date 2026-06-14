package com.gestioninmobiliaria.data.remote

import com.gestioninmobiliaria.data.model.Pago
import retrofit2.http.GET

interface PagoApi {
    @GET("pagos")
    suspend fun getPagos(): List<Pago>
}
