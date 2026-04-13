package com.propmanager.core.network.api

import kotlinx.serialization.Serializable
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.PUT

@Serializable data class MonedaConfigDto(val tasa: Double, val actualizado: String)

@Serializable data class UpdateMonedaRequest(val tasa: Double)

interface ConfiguracionApiService {
    @GET("api/configuracion/moneda") suspend fun getMoneda(): Response<MonedaConfigDto>

    @PUT("api/configuracion/moneda")
    suspend fun updateMoneda(@Body request: UpdateMonedaRequest): Response<MonedaConfigDto>
}
