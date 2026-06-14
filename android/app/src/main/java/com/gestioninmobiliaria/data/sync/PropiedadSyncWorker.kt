package com.gestioninmobiliaria.data.sync

import android.content.Context
import androidx.hilt.work.HiltWorker
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import com.gestioninmobiliaria.data.repository.PropiedadRepository
import dagger.assisted.Assisted
import dagger.assisted.AssistedInject
import java.util.concurrent.TimeUnit

@HiltWorker
class PropiedadSyncWorker @AssistedInject constructor(
    @Assisted context: Context,
    @Assisted params: WorkerParameters,
    private val repository: PropiedadRepository,
) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result = try {
        repository.refresh()
        enqueue(WorkManager.getInstance(applicationContext))
        Result.success()
    } catch (_: Exception) {
        Result.retry()
    }

    companion object {
        private const val PERIODIC_WORK = "propiedad_sync_periodic"
        private const val ON_CONNECT_WORK = "propiedad_sync_on_connect"

        fun enqueue(workManager: WorkManager) {
            val connected = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build()

            val periodic = PeriodicWorkRequestBuilder<PropiedadSyncWorker>(1, TimeUnit.HOURS)
                .setConstraints(connected)
                .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 30, TimeUnit.SECONDS)
                .build()
            workManager.enqueueUniquePeriodicWork(
                PERIODIC_WORK,
                ExistingPeriodicWorkPolicy.KEEP,
                periodic,
            )

            val onConnect = OneTimeWorkRequestBuilder<PropiedadSyncWorker>()
                .setConstraints(connected)
                .build()
            workManager.enqueueUniqueWork(
                ON_CONNECT_WORK,
                ExistingWorkPolicy.REPLACE,
                onConnect,
            )
        }
    }
}
