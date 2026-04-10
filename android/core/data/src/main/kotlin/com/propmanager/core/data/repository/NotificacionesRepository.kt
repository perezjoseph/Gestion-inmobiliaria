package com.propmanager.core.data.repository

import com.propmanager.core.common.EmptyResponseException
import com.propmanager.core.network.api.NotificacionesApiService
import com.propmanager.core.network.api.PagoVencidoDto
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class NotificacionesRepository
@Inject
constructor(private val apiService: NotificacionesApiService) {
    suspend fun fetchPagosVencidos(): Result<List<PagoVencidoDto>> = runCatching {
        apiService.pagosVencidos().body()
            ?: throw EmptyResponseException("notificaciones/pagos-vencidos")
    }
}
