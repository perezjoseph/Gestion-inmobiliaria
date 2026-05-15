package com.propmanager.core.network.api

import com.propmanager.core.model.dto.PaginatedResponse
import com.propmanager.core.model.dto.UserDto
import kotlinx.serialization.Serializable
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.PUT
import retrofit2.http.Path
import retrofit2.http.Query

@Serializable
data class ChangeRoleRequest(val rol: String)

interface UsuariosApiService {
    @GET("api/usuarios")
    suspend fun getUsuarios(
        @Query("page") page: Int,
        @Query("per_page") perPage: Int,
    ): Response<PaginatedResponse<UserDto>>

    @PUT("api/usuarios/{id}/rol")
    suspend fun changeRole(
        @Path("id") id: String,
        @Body request: ChangeRoleRequest,
    ): Response<UserDto>

    @PUT("api/usuarios/{id}/toggle-activo")
    suspend fun toggleActivo(@Path("id") id: String): Response<UserDto>
}
