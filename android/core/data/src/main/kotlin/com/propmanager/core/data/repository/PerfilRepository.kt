package com.propmanager.core.data.repository

import com.propmanager.core.common.EmptyResponseException
import com.propmanager.core.model.dto.UserDto
import com.propmanager.core.network.api.ChangePasswordRequest
import com.propmanager.core.network.api.MessageResponse
import com.propmanager.core.network.api.PerfilApiService
import com.propmanager.core.network.api.UpdatePerfilRequest
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class PerfilRepository @Inject constructor(private val apiService: PerfilApiService) {
    suspend fun fetchPerfil(): Result<UserDto> = runCatching {
        apiService.getPerfil().body() ?: throw EmptyResponseException("perfil")
    }

    suspend fun updatePerfil(request: UpdatePerfilRequest): Result<UserDto> = runCatching {
        apiService.updatePerfil(request).body() ?: throw EmptyResponseException("perfil/update")
    }

    suspend fun changePassword(request: ChangePasswordRequest): Result<MessageResponse> =
        runCatching {
            apiService.changePassword(request).body()
                ?: throw EmptyResponseException("perfil/password")
        }
}
