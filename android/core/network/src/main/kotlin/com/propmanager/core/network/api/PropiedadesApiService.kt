package com.propmanager.core.network.api

import com.propmanager.core.model.dto.CreatePropiedadRequest
import com.propmanager.core.model.dto.PaginatedResponse
import com.propmanager.core.model.dto.PropiedadDto
import com.propmanager.core.model.dto.UpdatePropiedadRequest
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.PUT
import retrofit2.http.Path
import retrofit2.http.QueryMap

interface PropiedadesApiService {
    @GET("api/propiedades")
    suspend fun list(
        @QueryMap filters: Map<String, String> = emptyMap()
    ): Response<PaginatedResponse<PropiedadDto>>

    @GET("api/propiedades/{id}") suspend fun getById(@Path("id") id: String): Response<PropiedadDto>

    @POST("api/propiedades")
    suspend fun create(@Body request: CreatePropiedadRequest): Response<PropiedadDto>

    @PUT("api/propiedades/{id}")
    suspend fun update(
        @Path("id") id: String,
        @Body request: UpdatePropiedadRequest,
    ): Response<PropiedadDto>

    @DELETE("api/propiedades/{id}") suspend fun delete(@Path("id") id: String): Response<Unit>
}
