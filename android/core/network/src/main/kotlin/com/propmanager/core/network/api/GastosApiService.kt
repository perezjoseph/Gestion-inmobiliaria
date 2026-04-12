package com.propmanager.core.network.api

import com.propmanager.core.model.dto.CreateGastoRequest
import com.propmanager.core.model.dto.GastoDto
import com.propmanager.core.model.dto.PaginatedResponse
import com.propmanager.core.model.dto.UpdateGastoRequest
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.PUT
import retrofit2.http.Path
import retrofit2.http.QueryMap

interface GastosApiService {

    @GET("api/gastos")
    suspend fun list(@QueryMap filters: Map<String, String> = emptyMap()): Response<PaginatedResponse<GastoDto>>

    @POST("api/gastos")
    suspend fun create(@Body request: CreateGastoRequest): Response<GastoDto>

    @PUT("api/gastos/{id}")
    suspend fun update(@Path("id") id: String, @Body request: UpdateGastoRequest): Response<GastoDto>

    @DELETE("api/gastos/{id}")
    suspend fun delete(@Path("id") id: String): Response<Unit>

    @GET("api/gastos/resumen-categorias")
    suspend fun resumenCategorias(@QueryMap filters: Map<String, String> = emptyMap()): Response<List<Map<String, String>>>
}
