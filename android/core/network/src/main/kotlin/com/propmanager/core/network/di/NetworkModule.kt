package com.propmanager.core.network.di

import com.propmanager.core.network.AuthInterceptor
import com.propmanager.core.network.ConnectivityObserver
import com.propmanager.core.network.EncryptedTokenProvider
import com.propmanager.core.network.NetworkMonitor
import com.propmanager.core.network.TokenProvider
import com.propmanager.core.network.api.AuditoriaApiService
import com.propmanager.core.network.api.AuthApiService
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
import dagger.Binds
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import retrofit2.Retrofit
import retrofit2.converter.kotlinx.serialization.asConverterFactory
import java.util.concurrent.TimeUnit
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
abstract class TokenProviderModule {
    @Binds
    @Singleton
    abstract fun bindTokenProvider(impl: EncryptedTokenProvider): TokenProvider

    @Binds
    @Singleton
    abstract fun bindConnectivityObserver(impl: NetworkMonitor): ConnectivityObserver
}

@Module
@InstallIn(SingletonComponent::class)
object NetworkModule {
    private const val BASE_URL = "http://10.0.2.2:8080/"
    private const val CONNECT_TIMEOUT_SECONDS = 30L
    private const val READ_TIMEOUT_SECONDS = 30L
    private const val WRITE_TIMEOUT_SECONDS = 30L

    @Provides
    @Singleton
    fun provideJson(): Json =
        Json {
            ignoreUnknownKeys = true
            coerceInputValues = true
            encodeDefaults = true
        }

    @Provides
    @Singleton
    fun provideOkHttpClient(authInterceptor: AuthInterceptor): OkHttpClient {
        val loggingInterceptor =
            HttpLoggingInterceptor().apply {
                level = HttpLoggingInterceptor.Level.BODY
            }
        return OkHttpClient
            .Builder()
            .addInterceptor(authInterceptor)
            .addInterceptor(loggingInterceptor)
            .connectTimeout(CONNECT_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            .readTimeout(READ_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            .writeTimeout(WRITE_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            .build()
    }

    @Provides
    @Singleton
    fun provideRetrofit(
        okHttpClient: OkHttpClient,
        json: Json,
    ): Retrofit {
        val contentType = "application/json".toMediaType()
        return Retrofit
            .Builder()
            .baseUrl(BASE_URL)
            .client(okHttpClient)
            .addConverterFactory(json.asConverterFactory(contentType))
            .build()
    }

    @Provides
    @Singleton
    fun provideAuthApiService(retrofit: Retrofit): AuthApiService = retrofit.create(AuthApiService::class.java)

    @Provides
    @Singleton
    fun providePropiedadesApiService(retrofit: Retrofit): PropiedadesApiService = retrofit.create(PropiedadesApiService::class.java)

    @Provides
    @Singleton
    fun provideInquilinosApiService(retrofit: Retrofit): InquilinosApiService = retrofit.create(InquilinosApiService::class.java)

    @Provides
    @Singleton
    fun provideContratosApiService(retrofit: Retrofit): ContratosApiService = retrofit.create(ContratosApiService::class.java)

    @Provides
    @Singleton
    fun providePagosApiService(retrofit: Retrofit): PagosApiService = retrofit.create(PagosApiService::class.java)

    @Provides
    @Singleton
    fun provideGastosApiService(retrofit: Retrofit): GastosApiService = retrofit.create(GastosApiService::class.java)

    @Provides
    @Singleton
    fun provideMantenimientoApiService(retrofit: Retrofit): MantenimientoApiService = retrofit.create(MantenimientoApiService::class.java)

    @Provides
    @Singleton
    fun provideDashboardApiService(retrofit: Retrofit): DashboardApiService = retrofit.create(DashboardApiService::class.java)

    @Provides
    @Singleton
    fun provideReportesApiService(retrofit: Retrofit): ReportesApiService = retrofit.create(ReportesApiService::class.java)

    @Provides
    @Singleton
    fun provideDocumentosApiService(retrofit: Retrofit): DocumentosApiService = retrofit.create(DocumentosApiService::class.java)

    @Provides
    @Singleton
    fun provideNotificacionesApiService(retrofit: Retrofit): NotificacionesApiService =
        retrofit.create(NotificacionesApiService::class.java)

    @Provides
    @Singleton
    fun provideAuditoriaApiService(retrofit: Retrofit): AuditoriaApiService = retrofit.create(AuditoriaApiService::class.java)

    @Provides
    @Singleton
    fun provideConfiguracionApiService(retrofit: Retrofit): ConfiguracionApiService = retrofit.create(ConfiguracionApiService::class.java)

    @Provides
    @Singleton
    fun provideImportacionApiService(retrofit: Retrofit): ImportacionApiService = retrofit.create(ImportacionApiService::class.java)

    @Provides
    @Singleton
    fun providePerfilApiService(retrofit: Retrofit): PerfilApiService = retrofit.create(PerfilApiService::class.java)
}
