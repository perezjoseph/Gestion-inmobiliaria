package com.propmanager.core.network.api

import com.propmanager.core.model.dto.CreateInquilinoRequest
import com.propmanager.core.model.dto.InquilinoDto
import com.propmanager.core.model.dto.PaginatedResponse
import com.propmanager.core.model.dto.UpdateInquilinoRequest
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.PUT
import retrofit2.http.Path
import retrofit2.http.QueryMap

interface InquilinosApiService {
    @GET("api/inquilinos")
    suspend fun list(
        @QueryMap filters: Map<String, String> = emptyMap()
    ): Response<PaginatedResponse<InquilinoDto>>

    @GET("api/inquilinos/{id}") suspend fun getById(@Path("id") id: String): Response<InquilinoDto>

    @POST("api/inquilinos")
    suspend fun create(@Body request: CreateInquilinoRequest): Response<InquilinoDto>

    @PUT("api/inquilinos/{id}")
    suspend fun update(
        @Path("id") id: String,
        @Body request: UpdateInquilinoRequest,
    ): Response<InquilinoDto>

    @DELETE("api/inquilinos/{id}") suspend fun delete(@Path("id") id: String): Response<Unit>
}
