package com.propmanager.core.data.di

import android.content.Context
import androidx.work.WorkManager
import com.propmanager.core.data.repository.AuditoriaRepository
import com.propmanager.core.data.repository.ConfiguracionRepository
import com.propmanager.core.data.repository.ContratosRepository
import com.propmanager.core.data.repository.DashboardRepository
import com.propmanager.core.data.repository.DocumentosRepository
import com.propmanager.core.data.repository.GastosRepository
import com.propmanager.core.data.repository.ImportacionRepository
import com.propmanager.core.data.repository.InquilinosRepository
import com.propmanager.core.data.repository.MantenimientoRepository
import com.propmanager.core.data.repository.NotificacionesRepository
import com.propmanager.core.data.repository.PagosRepository
import com.propmanager.core.data.repository.PerfilRepository
import com.propmanager.core.data.repository.PropiedadesRepository
import com.propmanager.core.data.repository.ReportesRepository
import com.propmanager.core.data.sync.SyncManager
import com.propmanager.core.database.dao.ContratoDao
import com.propmanager.core.database.dao.DashboardCacheDao
import com.propmanager.core.database.dao.GastoDao
import com.propmanager.core.database.dao.InquilinoDao
import com.propmanager.core.database.dao.NotaMantenimientoDao
import com.propmanager.core.database.dao.PagoDao
import com.propmanager.core.database.dao.PropiedadDao
import com.propmanager.core.database.dao.SolicitudMantenimientoDao
import com.propmanager.core.database.dao.SyncQueueDao
import com.propmanager.core.network.api.AuditoriaApiService
import com.propmanager.core.network.api.ConfiguracionApiService
import com.propmanager.core.network.api.ContratosApiService
import com.propmanager.core.network.api.DashboardApiService
import com.propmanager.core.network.api.DocumentosApiService
import com.propmanager.core.network.api.GastosApiService
import com.propmanager.core.network.api.ImportacionApiService
import com.propmanager.core.network.api.InquilinosApiService
import com.propmanager.core.network.api.MantenimientoApiService
import com.propmanager.core.network.api.NotificacionesApiService
import com.propmanager.core.network.api.PagosApiService
import com.propmanager.core.network.api.PerfilApiService
import com.propmanager.core.network.api.PropiedadesApiService
import com.propmanager.core.network.api.ReportesApiService
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import kotlinx.serialization.json.Json
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object DataModule {
    @Provides
    @Singleton
    fun provideJson(): Json =
        Json {
            ignoreUnknownKeys = true
            encodeDefaults = true
        }

    @Provides
    @Singleton
    fun provideWorkManager(
        @ApplicationContext context: Context,
    ): WorkManager = WorkManager.getInstance(context)

    @Provides
    @Singleton
    fun provideSyncManager(workManager: WorkManager): SyncManager = SyncManager(workManager)

    @Provides
    @Singleton
    fun providePropiedadesRepository(
        dao: PropiedadDao,
        syncQueueDao: SyncQueueDao,
        apiService: PropiedadesApiService,
        json: Json,
    ): PropiedadesRepository = PropiedadesRepository(dao, syncQueueDao, apiService, json)

    @Provides
    @Singleton
    fun provideInquilinosRepository(
        dao: InquilinoDao,
        syncQueueDao: SyncQueueDao,
        apiService: InquilinosApiService,
        json: Json,
    ): InquilinosRepository = InquilinosRepository(dao, syncQueueDao, apiService, json)

    @Provides
    @Singleton
    fun provideContratosRepository(
        dao: ContratoDao,
        syncQueueDao: SyncQueueDao,
        apiService: ContratosApiService,
        json: Json,
    ): ContratosRepository = ContratosRepository(dao, syncQueueDao, apiService, json)

    @Provides
    @Singleton
    fun providePagosRepository(
        dao: PagoDao,
        syncQueueDao: SyncQueueDao,
        apiService: PagosApiService,
        json: Json,
    ): PagosRepository = PagosRepository(dao, syncQueueDao, apiService, json)

    @Provides
    @Singleton
    fun provideGastosRepository(
        dao: GastoDao,
        syncQueueDao: SyncQueueDao,
        apiService: GastosApiService,
        json: Json,
    ): GastosRepository = GastosRepository(dao, syncQueueDao, apiService, json)

    @Provides
    @Singleton
    fun provideMantenimientoRepository(
        solicitudDao: SolicitudMantenimientoDao,
        notaDao: NotaMantenimientoDao,
        syncQueueDao: SyncQueueDao,
        apiService: MantenimientoApiService,
        json: Json,
    ): MantenimientoRepository = MantenimientoRepository(solicitudDao, notaDao, syncQueueDao, apiService, json)

    @Provides
    @Singleton
    fun provideDashboardRepository(
        apiService: DashboardApiService,
        cacheDao: DashboardCacheDao,
        json: Json,
    ): DashboardRepository = DashboardRepository(apiService, cacheDao, json)

    @Provides
    @Singleton
    fun provideReportesRepository(apiService: ReportesApiService): ReportesRepository = ReportesRepository(apiService)

    @Provides
    @Singleton
    fun provideDocumentosRepository(apiService: DocumentosApiService): DocumentosRepository = DocumentosRepository(apiService)

    @Provides
    @Singleton
    fun provideNotificacionesRepository(apiService: NotificacionesApiService): NotificacionesRepository =
        NotificacionesRepository(apiService)

    @Provides
    @Singleton
    fun provideAuditoriaRepository(apiService: AuditoriaApiService): AuditoriaRepository = AuditoriaRepository(apiService)

    @Provides
    @Singleton
    fun providePerfilRepository(apiService: PerfilApiService): PerfilRepository = PerfilRepository(apiService)

    @Provides
    @Singleton
    fun provideConfiguracionRepository(apiService: ConfiguracionApiService): ConfiguracionRepository = ConfiguracionRepository(apiService)

    @Provides
    @Singleton
    fun provideImportacionRepository(apiService: ImportacionApiService): ImportacionRepository = ImportacionRepository(apiService)
}
