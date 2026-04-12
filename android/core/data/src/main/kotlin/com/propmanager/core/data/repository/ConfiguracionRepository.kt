package com.propmanager.core.data.repository

import com.propmanager.core.network.api.ConfiguracionApiService
import com.propmanager.core.network.api.MonedaConfigDto
import com.propmanager.core.network.api.UpdateMonedaRequest
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ConfiguracionRepository
    @Inject
    constructor(
        private val apiService: ConfiguracionApiService,
    ) {
        suspend fun fetchMoneda(): Result<MonedaConfigDto> =
            runCatching {
                apiService.getMoneda().body() ?: throw Exception("Empty response")
            }

        suspend fun updateMoneda(request: UpdateMonedaRequest): Result<MonedaConfigDto> =
            runCatching {
                apiService.updateMoneda(request).body() ?: throw Exception("Empty response")
            }
    }
