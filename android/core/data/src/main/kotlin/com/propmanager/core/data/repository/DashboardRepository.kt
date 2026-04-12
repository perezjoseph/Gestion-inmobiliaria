package com.propmanager.core.data.repository

import com.propmanager.core.database.dao.DashboardCacheDao
import com.propmanager.core.database.entity.DashboardCache
import com.propmanager.core.model.dto.ContratoCalendario
import com.propmanager.core.model.dto.DashboardStats
import com.propmanager.core.model.dto.GastosComparacion
import com.propmanager.core.model.dto.IngresosComparacion
import com.propmanager.core.model.dto.OcupacionTendencia
import com.propmanager.core.model.dto.PagoProximo
import com.propmanager.core.network.api.DashboardApiService
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.time.Instant
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class DashboardRepository @Inject constructor(
    private val apiService: DashboardApiService,
    private val cacheDao: DashboardCacheDao,
    private val json: Json
) {

    suspend fun fetchStats(): Result<DashboardStats> =
        fetchAndCache("stats") { apiService.stats() }

    suspend fun fetchPagosProximos(): Result<List<PagoProximo>> =
        fetchAndCache("pagos_proximos") { apiService.pagosProximos() }

    suspend fun fetchContratosCalendario(): Result<List<ContratoCalendario>> =
        fetchAndCache("contratos_calendario") { apiService.contratosCalendario() }

    suspend fun fetchOcupacionTendencia(): Result<List<OcupacionTendencia>> =
        fetchAndCache("ocupacion_tendencia") { apiService.ocupacionTendencia() }

    suspend fun fetchIngresosComparacion(): Result<IngresosComparacion> =
        fetchAndCache("ingresos_comparacion") { apiService.ingresosComparacion() }

    suspend fun fetchGastosComparacion(): Result<GastosComparacion> =
        fetchAndCache("gastos_comparacion") { apiService.gastosComparacion() }

    suspend fun getCachedStats(): DashboardStats? = getCached("stats")
    suspend fun getCachedPagosProximos(): List<PagoProximo>? = getCached("pagos_proximos")
    suspend fun getCachedContratosCalendario(): List<ContratoCalendario>? = getCached("contratos_calendario")
    suspend fun getCachedOcupacionTendencia(): List<OcupacionTendencia>? = getCached("ocupacion_tendencia")
    suspend fun getCachedIngresosComparacion(): IngresosComparacion? = getCached("ingresos_comparacion")
    suspend fun getCachedGastosComparacion(): GastosComparacion? = getCached("gastos_comparacion")

    suspend fun getCachedAt(key: String): Instant? =
        cacheDao.getByKey(key)?.cachedAt?.let { Instant.ofEpochMilli(it) }

    private suspend inline fun <reified T> fetchAndCache(
        key: String,
        crossinline apiCall: suspend () -> retrofit2.Response<T>
    ): Result<T> = runCatching {
        val response = apiCall()
        val body = response.body() ?: throw Exception("Empty response for $key")
        cacheDao.upsert(
            DashboardCache(
                key = key,
                data = json.encodeToString(body),
                cachedAt = Instant.now().toEpochMilli()
            )
        )
        body
    }

    private suspend inline fun <reified T> getCached(key: String): T? {
        val cache = cacheDao.getByKey(key) ?: return null
        return try {
            json.decodeFromString<T>(cache.data)
        } catch (_: Exception) {
            null
        }
    }
}
