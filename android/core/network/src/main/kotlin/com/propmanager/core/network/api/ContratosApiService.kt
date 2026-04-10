package com.propmanager.core.network.api

import com.propmanager.core.model.dto.ContratoDto
import com.propmanager.core.model.dto.CreateContratoRequest
import com.propmanager.core.model.dto.PaginatedResponse
import com.propmanager.core.model.dto.RenovarContratoRequest
import com.propmanager.core.model.dto.TerminarContratoRequest
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.PUT
import retrofit2.http.Path
import retrofit2.http.QueryMap

interface ContratosApiService {
    @GET("api/contratos")
    suspend fun list(
        @QueryMap filters: Map<String, String> = emptyMap()
    ): Response<PaginatedResponse<ContratoDto>>

    @GET("api/contratos/{id}") suspend fun getById(@Path("id") id: String): Response<ContratoDto>

    @POST("api/contratos")
    suspend fun create(@Body request: CreateContratoRequest): Response<ContratoDto>

    @PUT("api/contratos/{id}")
    suspend fun update(
        @Path("id") id: String,
        @Body request: CreateContratoRequest,
    ): Response<ContratoDto>

    @DELETE("api/contratos/{id}") suspend fun delete(@Path("id") id: String): Response<Unit>

    @POST("api/contratos/{id}/renovar")
    suspend fun renovar(
        @Path("id") id: String,
        @Body request: RenovarContratoRequest,
    ): Response<ContratoDto>

    @POST("api/contratos/{id}/terminar")
    suspend fun terminar(
        @Path("id") id: String,
        @Body request: TerminarContratoRequest,
    ): Response<ContratoDto>

    @GET("api/contratos/por-vencer")
    suspend fun expiring(
        @QueryMap filters: Map<String, String> = emptyMap()
    ): Response<List<ContratoDto>>
}
