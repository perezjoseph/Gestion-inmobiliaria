package com.propmanager.core.network.api

import com.propmanager.core.model.dto.UserDto
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.PUT

@Serializable
data class UpdatePerfilRequest(
    val nombre: String? = null,
    val email: String? = null
)

@Serializable
data class ChangePasswordRequest(
    @SerialName("passwordActual") val passwordActual: String,
    @SerialName("passwordNuevo") val passwordNuevo: String
)

@Serializable
data class MessageResponse(
    val message: String
)

interface PerfilApiService {

    @GET("api/perfil")
    suspend fun getPerfil(): Response<UserDto>

    @PUT("api/perfil")
    suspend fun updatePerfil(@Body request: UpdatePerfilRequest): Response<UserDto>

    @PUT("api/perfil/password")
    suspend fun changePassword(@Body request: ChangePasswordRequest): Response<MessageResponse>
}
