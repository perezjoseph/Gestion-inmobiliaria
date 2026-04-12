package com.propmanager.core.network.api

import com.propmanager.core.model.dto.PaginatedResponse
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement
import retrofit2.Response
import retrofit2.http.GET
import retrofit2.http.QueryMap

@Serializable
data class AuditoriaDto(
    val id: String,
    @SerialName("usuarioId") val usuarioId: String,
    @SerialName("entityType") val entityType: String,
    @SerialName("entityId") val entityId: String,
    val accion: String,
    val cambios: JsonElement,
    @SerialName("createdAt") val createdAt: String
)

interface AuditoriaApiService {

    @GET("api/auditoria")
    suspend fun list(@QueryMap filters: Map<String, String> = emptyMap()): Response<PaginatedResponse<AuditoriaDto>>
}
