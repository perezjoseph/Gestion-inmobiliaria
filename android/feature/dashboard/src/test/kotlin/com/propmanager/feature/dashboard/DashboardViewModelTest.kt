package com.propmanager.feature.dashboard

import com.propmanager.core.data.repository.DashboardRepository
import com.propmanager.core.model.dto.ContratoCalendario
import com.propmanager.core.model.dto.DashboardStats
import com.propmanager.core.model.dto.GastosComparacion
import com.propmanager.core.model.dto.IngresosComparacion
import com.propmanager.core.model.dto.OcupacionTendencia
import com.propmanager.core.model.dto.PagoProximo
import com.propmanager.core.network.ConnectivityObserver
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.nulls.shouldBeNull
import io.kotest.matchers.nulls.shouldNotBeNull
import io.kotest.matchers.shouldBe
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import java.time.Instant

/**
 * Validates: Requirements 9.1, 9.7
 */
@OptIn(ExperimentalCoroutinesApi::class)
class DashboardViewModelTest :
    FreeSpec({

        val testDispatcher = StandardTestDispatcher()

        beforeEach { Dispatchers.setMain(testDispatcher) }
        afterEach { Dispatchers.resetMain() }

        "online data fetch populates all dashboard sections" {
            val stats =
                DashboardStats(
                    totalPropiedades = 12,
                    tasaOcupacion = 85.5,
                    ingresoMensual = "150000.00",
                    pagosAtrasados = 3,
                    totalGastosMes = "45000.00",
                )
            val pagos =
                listOf(
                    PagoProximo("p1", "Apartamento Centro", "Juan García", "25000.00", "DOP", "2025-07-15"),
                )
            val contratos =
                listOf(
                    ContratoCalendario("c1", "Casa Norte", "María López", "2025-08-01", 30, "yellow"),
                )
            val ocupacion = listOf(OcupacionTendencia(mes = 6, anio = 2025, tasa = 90.0))
            val ingresos = IngresosComparacion("200000.00", "175000.00", "-25000.00")
            val gastos = GastosComparacion("45000.00", "38000.00", 18.4)

            val repo =
                FakeDashboardRepository(
                    statsResult = Result.success(stats),
                    pagosResult = Result.success(pagos),
                    contratosResult = Result.success(contratos),
                    ocupacionResult = Result.success(ocupacion),
                    ingresosResult = Result.success(ingresos),
                    gastosResult = Result.success(gastos),
                )

            runTest(testDispatcher) {
                val vm = DashboardViewModel(repo, FakeConnectivityObserver(online = true))
                advanceUntilIdle()

                val state = vm.uiState.value
                state.isLoading shouldBe false
                state.errorMessage.shouldBeNull()
                state.isFromCache shouldBe false
                state.lastUpdated.shouldBeNull()

                state.stats.shouldNotBeNull()
                state.stats!!.totalPropiedades shouldBe 12
                state.stats!!.pagosAtrasados shouldBe 3
                state.pagosProximos.size shouldBe 1
                state.pagosProximos[0].propiedadTitulo shouldBe "Apartamento Centro"
                state.contratosCalendario.size shouldBe 1
                state.contratosCalendario[0].diasRestantes shouldBe 30
                state.ocupacionTendencia.size shouldBe 1
                state.ingresosComparacion.shouldNotBeNull()
                state.ingresosComparacion!!.cobrado shouldBe "175000.00"
                state.gastosComparacion.shouldNotBeNull()
            }
        }

        "offline with cached data shows staleness indicator" {
            val cachedStats =
                DashboardStats(
                    totalPropiedades = 10,
                    tasaOcupacion = 80.0,
                    ingresoMensual = "120000.00",
                    pagosAtrasados = 2,
                    totalGastosMes = "30000.00",
                )
            val cachedPagos =
                listOf(
                    PagoProximo("p2", "Local Comercial", "Pedro Martínez", "15000.00", "DOP", "2025-07-10"),
                )
            val cachedContratos =
                listOf(
                    ContratoCalendario("c2", "Oficina Este", "Ana Rodríguez", "2025-09-15", 75, "green"),
                )
            val cachedAt = Instant.parse("2025-07-01T14:30:00Z")

            val repo =
                FakeDashboardRepository(
                    cachedStats = cachedStats,
                    cachedPagos = cachedPagos,
                    cachedContratos = cachedContratos,
                    cachedAtInstant = cachedAt,
                )

            runTest(testDispatcher) {
                val vm = DashboardViewModel(repo, FakeConnectivityObserver(online = false))
                advanceUntilIdle()

                val state = vm.uiState.value
                state.isLoading shouldBe false
                state.errorMessage.shouldBeNull()
                state.isFromCache shouldBe true
                state.lastUpdated.shouldNotBeNull()
                state.stats.shouldNotBeNull()
                state.stats!!.totalPropiedades shouldBe 10
                state.pagosProximos.size shouldBe 1
                state.contratosCalendario.size shouldBe 1
            }
        }

        "offline with no cached data shows error message" {
            val repo = FakeDashboardRepository()

            runTest(testDispatcher) {
                val vm = DashboardViewModel(repo, FakeConnectivityObserver(online = false))
                advanceUntilIdle()

                val state = vm.uiState.value
                state.isLoading shouldBe false
                state.errorMessage.shouldNotBeNull()
                state.stats.shouldBeNull()
                state.isFromCache shouldBe false
            }
        }

        "network failure falls back to cache with staleness" {
            val cachedStats =
                DashboardStats(
                    totalPropiedades = 5,
                    tasaOcupacion = 60.0,
                    ingresoMensual = "80000.00",
                    pagosAtrasados = 1,
                    totalGastosMes = "20000.00",
                )
            val cachedAt = Instant.parse("2025-06-30T10:00:00Z")

            val repo =
                FakeDashboardRepository(
                    statsResult = Result.failure(Exception("Network error")),
                    pagosResult = Result.failure(Exception("Network error")),
                    contratosResult = Result.failure(Exception("Network error")),
                    cachedStats = cachedStats,
                    cachedContratos = emptyList(),
                    cachedPagos = emptyList(),
                    cachedAtInstant = cachedAt,
                )

            runTest(testDispatcher) {
                val vm = DashboardViewModel(repo, FakeConnectivityObserver(online = true))
                advanceUntilIdle()

                val state = vm.uiState.value
                state.isLoading shouldBe false
                state.isFromCache shouldBe true
                state.stats.shouldNotBeNull()
                state.stats!!.totalPropiedades shouldBe 5
                state.lastUpdated.shouldNotBeNull()
            }
        }
    })

private class FakeConnectivityObserver(
    online: Boolean,
) : ConnectivityObserver {
    override val isOnline: StateFlow<Boolean> = MutableStateFlow(online).asStateFlow()
}

private class FakeDashboardRepository(
    private val statsResult: Result<DashboardStats> = Result.failure(Exception("no data")),
    private val pagosResult: Result<List<PagoProximo>> = Result.failure(Exception("no data")),
    private val contratosResult: Result<List<ContratoCalendario>> = Result.failure(Exception("no data")),
    private val ocupacionResult: Result<List<OcupacionTendencia>> = Result.failure(Exception("no data")),
    private val ingresosResult: Result<IngresosComparacion> = Result.failure(Exception("no data")),
    private val gastosResult: Result<GastosComparacion> = Result.failure(Exception("no data")),
    private val cachedStats: DashboardStats? = null,
    private val cachedPagos: List<PagoProximo>? = null,
    private val cachedContratos: List<ContratoCalendario>? = null,
    private val cachedOcupacion: List<OcupacionTendencia>? = null,
    private val cachedIngresos: IngresosComparacion? = null,
    private val cachedGastos: GastosComparacion? = null,
    private val cachedAtInstant: Instant? = null,
) : DashboardRepository(
        apiService = StubDashboardApiService,
        cacheDao = StubDashboardCacheDao,
        json = kotlinx.serialization.json.Json,
    ) {
    override suspend fun fetchStats() = statsResult

    override suspend fun fetchPagosProximos() = pagosResult

    override suspend fun fetchContratosCalendario() = contratosResult

    override suspend fun fetchOcupacionTendencia() = ocupacionResult

    override suspend fun fetchIngresosComparacion() = ingresosResult

    override suspend fun fetchGastosComparacion() = gastosResult

    override suspend fun getCachedStats() = cachedStats

    override suspend fun getCachedPagosProximos() = cachedPagos

    override suspend fun getCachedContratosCalendario() = cachedContratos

    override suspend fun getCachedOcupacionTendencia() = cachedOcupacion

    override suspend fun getCachedIngresosComparacion() = cachedIngresos

    override suspend fun getCachedGastosComparacion() = cachedGastos

    override suspend fun getCachedAt(key: String) = cachedAtInstant
}

private object StubDashboardApiService : com.propmanager.core.network.api.DashboardApiService {
    override suspend fun stats(): retrofit2.Response<DashboardStats> = error("stub")

    override suspend fun pagosProximos(): retrofit2.Response<List<PagoProximo>> = error("stub")

    override suspend fun contratosCalendario(): retrofit2.Response<List<ContratoCalendario>> = error("stub")

    override suspend fun ocupacionTendencia(): retrofit2.Response<List<OcupacionTendencia>> = error("stub")

    override suspend fun ingresosComparacion(): retrofit2.Response<IngresosComparacion> = error("stub")

    override suspend fun gastosComparacion(): retrofit2.Response<GastosComparacion> = error("stub")
}

private object StubDashboardCacheDao : com.propmanager.core.database.dao.DashboardCacheDao {
    override suspend fun getByKey(key: String) = null

    override suspend fun upsert(cache: com.propmanager.core.database.entity.DashboardCache) {}

    override suspend fun deleteAll() {}
}
