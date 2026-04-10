package com.propmanager

import android.app.Application
import com.propmanager.core.data.sync.SyncManager
import dagger.hilt.android.HiltAndroidApp
import javax.inject.Inject

@HiltAndroidApp
class PropManagerApp : Application() {
    @Inject lateinit var syncManager: SyncManager

    override fun onCreate() {
        super.onCreate()
        syncManager.schedulePeriodicRefresh()
        syncManager.scheduleSyncWorker()
    }
}
