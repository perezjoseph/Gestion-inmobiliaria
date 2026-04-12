package com.propmanager.core.network.api

import com.propmanager.core.model.dto.CreateNotaRequest
import com.propmanager.core.model.dto.CreateSolicitudRequest
import com.propmanager.core.model.dto.NotaDto
import com.propmanager.core.model.dto.PaginatedResponse
import com.propmanager.core.model.dto.SolicitudDto
import com.propmanager.core.model.dto.UpdateEstadoRequest
import com.propmanager.core.model.dto.UpdateSolicitudRequest
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.PUT
import retrofit2.http.Path
import retrofit2.http.QueryMap

interface MantenimientoApiService {

    @GET("api/mantenimiento")
    suspend fun list(@QueryMap filters: Map<String, String> = emptyMap()): Response<PaginatedResponse<SolicitudDto>>

    @GET("api/mantenimiento/{id}")
    suspend fun getById(@Path("id") id: String): Response<SolicitudDto>

    @POST("api/mantenimiento")
    suspend fun create(@Body request: CreateSolicitudRequest): Response<SolicitudDto>

    @PUT("api/mantenimiento/{id}")
    suspend fun update(@Path("id") id: String, @Body request: UpdateSolicitudRequest): Response<SolicitudDto>

    @DELETE("api/mantenimiento/{id}")
    suspend fun delete(@Path("id") id: String): Response<Unit>

    @PUT("api/mantenimiento/{id}/estado")
    suspend fun updateEstado(@Path("id") id: String, @Body request: UpdateEstadoRequest): Response<SolicitudDto>

    @POST("api/mantenimiento/{id}/notas")
    suspend fun addNota(@Path("id") id: String, @Body request: CreateNotaRequest): Response<NotaDto>
}
