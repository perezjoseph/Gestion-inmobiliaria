package com.propmanager.core.network.api

import com.propmanager.core.model.dto.CreatePagoRequest
import com.propmanager.core.model.dto.PaginatedResponse
import com.propmanager.core.model.dto.PagoDto
import com.propmanager.core.model.dto.UpdatePagoRequest
import okhttp3.ResponseBody
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.PUT
import retrofit2.http.Path
import retrofit2.http.QueryMap

interface PagosApiService {
    @GET("api/pagos")
    suspend fun list(
        @QueryMap filters: Map<String, String> = emptyMap(),
    ): Response<PaginatedResponse<PagoDto>>

    @POST("api/pagos")
    suspend fun create(
        @Body request: CreatePagoRequest,
    ): Response<PagoDto>

    @PUT("api/pagos/{id}")
    suspend fun update(
        @Path("id") id: String,
        @Body request: UpdatePagoRequest,
    ): Response<PagoDto>

    @DELETE("api/pagos/{id}")
    suspend fun delete(
        @Path("id") id: String,
    ): Response<Unit>

    @GET("api/pagos/{id}/recibo")
    suspend fun getRecibo(
        @Path("id") id: String,
    ): Response<ResponseBody>
}
