package com.propmanager.core.data.sync

import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import java.util.concurrent.TimeUnit
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class SyncManager @Inject constructor(private val workManager: WorkManager) {
    private val networkConstraint =
        Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()

    fun scheduleSyncWorker() {
        val request =
            OneTimeWorkRequestBuilder<SyncWorker>().setConstraints(networkConstraint).build()

        workManager.enqueueUniqueWork(SyncWorker.WORK_NAME, ExistingWorkPolicy.REPLACE, request)
    }

    fun schedulePeriodicRefresh(intervalMinutes: Long = DEFAULT_REFRESH_INTERVAL_MINUTES) {
        val request =
            PeriodicWorkRequestBuilder<PeriodicRefreshWorker>(intervalMinutes, TimeUnit.MINUTES)
                .setConstraints(networkConstraint)
                .build()

        workManager.enqueueUniquePeriodicWork(
            PeriodicRefreshWorker.WORK_NAME,
            ExistingPeriodicWorkPolicy.KEEP,
            request,
        )
    }

    fun cancelAll() {
        workManager.cancelUniqueWork(SyncWorker.WORK_NAME)
        workManager.cancelUniqueWork(PeriodicRefreshWorker.WORK_NAME)
    }

    companion object {
        const val DEFAULT_REFRESH_INTERVAL_MINUTES = 15L
    }
}
