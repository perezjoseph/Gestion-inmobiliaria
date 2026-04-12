package com.propmanager.core.data.repository

import com.propmanager.core.model.dto.PaginatedResponse
import com.propmanager.core.network.api.AuditoriaApiService
import com.propmanager.core.network.api.AuditoriaDto
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class AuditoriaRepository @Inject constructor(
    private val apiService: AuditoriaApiService
) {

    suspend fun fetchAuditLog(filters: Map<String, String> = emptyMap()): Result<PaginatedResponse<AuditoriaDto>> = runCatching {
        apiService.list(filters).body() ?: throw Exception("Empty response")
    }
}
