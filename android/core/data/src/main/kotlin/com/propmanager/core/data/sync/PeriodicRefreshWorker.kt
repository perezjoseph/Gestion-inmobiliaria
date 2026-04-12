package com.propmanager.core.data.sync

import android.content.Context
import android.util.Log
import androidx.hilt.work.HiltWorker
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import com.propmanager.core.data.repository.ContratosRepository
import com.propmanager.core.data.repository.GastosRepository
import com.propmanager.core.data.repository.InquilinosRepository
import com.propmanager.core.data.repository.MantenimientoRepository
import com.propmanager.core.data.repository.PagosRepository
import com.propmanager.core.data.repository.PropiedadesRepository
import dagger.assisted.Assisted
import dagger.assisted.AssistedInject

@HiltWorker
class PeriodicRefreshWorker @AssistedInject constructor(
    @Assisted context: Context,
    @Assisted params: WorkerParameters,
    private val propiedadesRepository: PropiedadesRepository,
    private val inquilinosRepository: InquilinosRepository,
    private val contratosRepository: ContratosRepository,
    private val pagosRepository: PagosRepository,
    private val gastosRepository: GastosRepository,
    private val mantenimientoRepository: MantenimientoRepository
) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result {
        var hasError = false

        listOf(
            "propiedades" to { propiedadesRepository.refreshFromServer() },
            "inquilinos" to { inquilinosRepository.refreshFromServer() },
            "contratos" to { contratosRepository.refreshFromServer() },
            "pagos" to { pagosRepository.refreshFromServer() },
            "gastos" to { gastosRepository.refreshFromServer() },
            "mantenimiento" to { mantenimientoRepository.refreshFromServer() }
        ).forEach { (name, refresh) ->
            refresh().onFailure { e ->
                Log.e(TAG, "Failed to refresh $name", e)
                hasError = true
            }
        }

        return if (hasError) Result.retry() else Result.success()
    }

    companion object {
        const val TAG = "PeriodicRefreshWorker"
        const val WORK_NAME = "periodic_refresh"
    }
}
