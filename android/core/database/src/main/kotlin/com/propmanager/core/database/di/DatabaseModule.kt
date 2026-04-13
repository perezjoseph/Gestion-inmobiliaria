package com.propmanager.core.database.di

import android.content.Context
import androidx.room.Room
import com.propmanager.core.database.PropManagerDatabase
import com.propmanager.core.database.dao.ContratoDao
import com.propmanager.core.database.dao.DashboardCacheDao
import com.propmanager.core.database.dao.GastoDao
import com.propmanager.core.database.dao.InquilinoDao
import com.propmanager.core.database.dao.NotaMantenimientoDao
import com.propmanager.core.database.dao.PagoDao
import com.propmanager.core.database.dao.PropiedadDao
import com.propmanager.core.database.dao.SolicitudMantenimientoDao
import com.propmanager.core.database.dao.SyncQueueDao
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object DatabaseModule {
    @Provides
    @Singleton
    fun provideDatabase(@ApplicationContext context: Context): PropManagerDatabase =
        Room.databaseBuilder(context, PropManagerDatabase::class.java, "propmanager.db").build()

    @Provides fun providePropiedadDao(db: PropManagerDatabase): PropiedadDao = db.propiedadDao()

    @Provides fun provideInquilinoDao(db: PropManagerDatabase): InquilinoDao = db.inquilinoDao()

    @Provides fun provideContratoDao(db: PropManagerDatabase): ContratoDao = db.contratoDao()

    @Provides fun providePagoDao(db: PropManagerDatabase): PagoDao = db.pagoDao()

    @Provides fun provideGastoDao(db: PropManagerDatabase): GastoDao = db.gastoDao()

    @Provides
    fun provideSolicitudDao(db: PropManagerDatabase): SolicitudMantenimientoDao = db.solicitudDao()

    @Provides fun provideNotaDao(db: PropManagerDatabase): NotaMantenimientoDao = db.notaDao()

    @Provides fun provideSyncQueueDao(db: PropManagerDatabase): SyncQueueDao = db.syncQueueDao()

    @Provides
    fun provideDashboardCacheDao(db: PropManagerDatabase): DashboardCacheDao =
        db.dashboardCacheDao()
}
