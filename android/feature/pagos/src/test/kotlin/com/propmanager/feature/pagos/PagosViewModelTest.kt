package com.propmanager.feature.pagos

import com.propmanager.core.data.repository.ContratosRepository
import com.propmanager.core.data.repository.PagosRepository
import com.propmanager.core.model.Contrato
import com.propmanager.core.model.Pago
import com.propmanager.core.model.dto.CreateContratoRequest
import com.propmanager.core.model.dto.CreatePagoRequest
import com.propmanager.core.model.dto.RenovarContratoRequest
import com.propmanager.core.model.dto.TerminarContratoRequest
import com.propmanager.core.model.dto.UpdatePagoRequest
import com.propmanager.core.network.ConnectivityObserver
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.collections.shouldHaveSize
import io.kotest.matchers.maps.shouldBeEmpty
import io.kotest.matchers.maps.shouldContainKey
import io.kotest.matchers.nulls.shouldBeNull
import io.kotest.matchers.nulls.shouldNotBeNull
import io.kotest.matchers.shouldBe
import io.kotest.matchers.string.shouldNotBeBlank
import io.kotest.matchers.types.shouldBeInstanceOf
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import kotlinx.serialization.json.Json
import java.lang.reflect.Proxy
import java.math.BigDecimal
import java.time.Instant
import java.time.LocalDate

/**
 * Unit tests for PagosViewModel.
 *
 * Validates: Requirements 6.1, 6.2, 6.7
 */
@OptIn(ExperimentalCoroutinesApi::class)
class PagosViewModelTest :
    FreeSpec({

        val testDispatcher = StandardTestDispatcher()

        beforeEach {
            Dispatchers.setMain(testDispatcher)
        }

        afterEach {
            Dispatchers.resetMain()
        }

        "list state" -
            {
                "emits Success with pagos from repository" {
                    val repo =
                        FakePagosRepository(
                            initialData = listOf(
                                samplePago("1", estado = "pendiente"),
                                samplePago("2", estado = "pagado"),
                            ),
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        val state = vm.pagos.value
                        state.shouldBeInstanceOf<PagosUiState.Success>()
                        state.pagos shouldHaveSize 2
                    }
                }

                "emits Success with empty list when no pagos" {
                    val repo = FakePagosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        val state = vm.pagos.value
                        state.shouldBeInstanceOf<PagosUiState.Success>()
                        state.pagos shouldHaveSize 0
                    }
                }
            }

        "filter application (Req 6.2)" -
            {
                "updateFilter filters by contratoId" {
                    val repo =
                        FakePagosRepository(
                            initialData = listOf(
                                samplePago("1", contratoId = "c1"),
                                samplePago("2", contratoId = "c2"),
                                samplePago("3", contratoId = "c1"),
                            ),
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(contratoId = "c1")
                        advanceUntilIdle()

                        vm.filters.value.contratoId shouldBe "c1"
                        val state = vm.pagos.value
                        state.shouldBeInstanceOf<PagosUiState.Success>()
                        state.pagos shouldHaveSize 2
                        state.pagos.all { it.contratoId == "c1" } shouldBe true
                    }
                }

                "updateFilter filters by estado" {
                    val repo =
                        FakePagosRepository(
                            initialData = listOf(
                                samplePago("1", estado = "pendiente"),
                                samplePago("2", estado = "pagado"),
                                samplePago("3", estado = "pendiente"),
                            ),
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(estado = "pendiente")
                        advanceUntilIdle()

                        val state = vm.pagos.value
                        state.shouldBeInstanceOf<PagosUiState.Success>()
                        state.pagos shouldHaveSize 2
                        state.pagos.all { it.estado == "pendiente" } shouldBe true
                    }
                }

                "updateFilter filters by fecha range" {
                    val repo =
                        FakePagosRepository(
                            initialData = listOf(
                                samplePago("1", fechaVencimiento = LocalDate.of(2025, 1, 15)),
                                samplePago("2", fechaVencimiento = LocalDate.of(2025, 3, 15)),
                                samplePago("3", fechaVencimiento = LocalDate.of(2025, 6, 15)),
                            ),
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(fechaDesde = "2025-01-01", fechaHasta = "2025-04-01")
                        advanceUntilIdle()

                        val state = vm.pagos.value
                        state.shouldBeInstanceOf<PagosUiState.Success>()
                        state.pagos shouldHaveSize 2
                    }
                }

                "clearFilters resets all filters and shows all pagos" {
                    val repo =
                        FakePagosRepository(
                            initialData = listOf(
                                samplePago("1", contratoId = "c1"),
                                samplePago("2", contratoId = "c2"),
                            ),
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(contratoId = "c1")
                        advanceUntilIdle()

                        vm.clearFilters()
                        advanceUntilIdle()

                        vm.filters.value shouldBe PagosFilterState()
                        val state = vm.pagos.value
                        state.shouldBeInstanceOf<PagosUiState.Success>()
                        state.pagos shouldHaveSize 2
                    }
                }
            }

        "CRUD state transitions" -
            {
                "create with valid data succeeds and calls onSuccess" {
                    val repo = FakePagosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("contratoId", "c1")
                        vm.onFieldChange("monto", "5000.00")
                        vm.onFechaVencimientoChange(LocalDate.of(2025, 6, 1))

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe true
                        vm.formState.value.errors.shouldBeEmpty()
                        vm.formState.value.isSubmitting shouldBe false
                        vm.successMessage.value shouldBe "Creado correctamente"
                        repo.createCallCount shouldBe 1
                    }
                }

                "update with valid data succeeds" {
                    val existing = samplePago("1")
                    val repo = FakePagosRepository(initialData = listOf(existing))
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.initEditForm(existing)
                        vm.onFieldChange("monto", "6000.00")

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe true
                        vm.successMessage.value shouldBe "Actualizado correctamente"
                        repo.updateCallCount shouldBe 1
                    }
                }

                "delete flow sets target, confirms, and clears" {
                    val target = samplePago("1")
                    val repo = FakePagosRepository(initialData = listOf(target))
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.requestDelete(target)
                        vm.deleteTarget.value.shouldNotBeNull()
                        vm.deleteTarget.value!!.id shouldBe "1"

                        vm.confirmDelete()
                        advanceUntilIdle()

                        vm.deleteTarget.value.shouldBeNull()
                        vm.successMessage.value shouldBe "Eliminado correctamente"
                        repo.deleteCallCount shouldBe 1
                    }
                }

                "dismissDelete clears delete target without deleting" {
                    val target = samplePago("1")
                    val repo = FakePagosRepository(initialData = listOf(target))
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.requestDelete(target)
                        vm.deleteTarget.value.shouldNotBeNull()

                        vm.dismissDelete()
                        vm.deleteTarget.value.shouldBeNull()
                        repo.deleteCallCount shouldBe 0
                    }
                }

                "create failure sets general error on form" {
                    val repo = FakePagosRepository(createError = RuntimeException("DB error"))
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("contratoId", "c1")
                        vm.onFieldChange("monto", "5000")
                        vm.onFechaVencimientoChange(LocalDate.of(2025, 6, 1))

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe false
                        vm.formState.value.errors shouldContainKey "general"
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }
            }

        "validation errors (Req 6.7)" -
            {
                "save with blank required fields shows validation errors and does not call repository" {
                    val repo = FakePagosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        val errors = vm.formState.value.errors
                        errors shouldContainKey "contratoId"
                        errors shouldContainKey "monto"
                        errors shouldContainKey "fechaVencimiento"
                        errors["contratoId"]!!.shouldNotBeBlank()
                        repo.createCallCount shouldBe 0
                    }
                }

                "onFieldChange clears error for that field" {
                    val repo = FakePagosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "contratoId"

                        vm.onFieldChange("contratoId", "c1")
                        vm.formState.value.errors.containsKey("contratoId") shouldBe false
                    }
                }

                "onFechaVencimientoChange clears fechaVencimiento error" {
                    val repo = FakePagosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "fechaVencimiento"
                        vm.onFechaVencimientoChange(LocalDate.of(2025, 6, 1))
                        vm.formState.value.errors.containsKey("fechaVencimiento") shouldBe false
                    }
                }
            }

        "form initialization" -
            {
                "initCreateForm resets form to defaults" {
                    val repo = FakePagosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.onFieldChange("contratoId", "c1")
                        vm.initCreateForm()

                        vm.formState.value.contratoId shouldBe ""
                        vm.formState.value.monto shouldBe ""
                        vm.formState.value.errors.shouldBeEmpty()
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }

                "initEditForm populates form from pago" {
                    val pago = samplePago(
                        "1",
                        contratoId = "c1",
                        monto = BigDecimal("7500.00"),
                        metodoPago = "transferencia",
                        notas = "Pago parcial",
                    )
                    val repo = FakePagosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(pagosRepo = repo)
                        advanceUntilIdle()

                        vm.initEditForm(pago)

                        vm.formState.value.contratoId shouldBe "c1"
                        vm.formState.value.monto shouldBe "7500.00"
                        vm.formState.value.metodoPago shouldBe "transferencia"
                        vm.formState.value.notas shouldBe "Pago parcial"
                        vm.formState.value.fechaVencimiento shouldBe pago.fechaVencimiento
                    }
                }
            }

        "contratos list is exposed for form selectors" {
            val contratosRepo = FakeContratosRepository(
                initialData = listOf(sampleContrato("c1"), sampleContrato("c2")),
            )
            val pagosRepo = FakePagosRepository()
            runTest(testDispatcher) {
                val vm = createViewModel(pagosRepo = pagosRepo, contratosRepo = contratosRepo)
                val job = backgroundScope.launch(testDispatcher) { vm.contratos.collect {} }
                advanceUntilIdle()

                vm.contratos.value shouldHaveSize 2
                job.cancel()
            }
        }

        "clearSuccessMessage resets success message" {
            val repo = FakePagosRepository(initialData = listOf(samplePago("1")))
            runTest(testDispatcher) {
                val vm = createViewModel(pagosRepo = repo)
                advanceUntilIdle()

                vm.requestDelete(samplePago("1"))
                vm.confirmDelete()
                advanceUntilIdle()

                vm.successMessage.value.shouldNotBeNull()
                vm.clearSuccessMessage()
                vm.successMessage.value.shouldBeNull()
            }
        }
    })

private fun createViewModel(
    pagosRepo: FakePagosRepository = FakePagosRepository(),
    contratosRepo: FakeContratosRepository = FakeContratosRepository(),
    connectivity: FakeConnectivityObserver = FakeConnectivityObserver(),
): PagosViewModel = PagosViewModel(pagosRepo, contratosRepo, connectivity)

private fun samplePago(
    id: String,
    contratoId: String = "c1",
    monto: BigDecimal = BigDecimal("5000.00"),
    moneda: String = "DOP",
    fechaPago: LocalDate? = null,
    fechaVencimiento: LocalDate = LocalDate.of(2025, 6, 1),
    metodoPago: String? = null,
    estado: String = "pendiente",
    notas: String? = null,
) = Pago(
    id = id,
    contratoId = contratoId,
    monto = monto,
    moneda = moneda,
    fechaPago = fechaPago,
    fechaVencimiento = fechaVencimiento,
    metodoPago = metodoPago,
    estado = estado,
    notas = notas,
    createdAt = Instant.now(),
    updatedAt = Instant.now(),
    isPendingSync = false,
)

private fun sampleContrato(
    id: String,
) = Contrato(
    id = id,
    propiedadId = "p1",
    inquilinoId = "i1",
    fechaInicio = LocalDate.of(2025, 1, 1),
    fechaFin = LocalDate.of(2025, 12, 31),
    montoMensual = BigDecimal("15000.00"),
    deposito = null,
    moneda = "DOP",
    estado = "activo",
    createdAt = Instant.now(),
    updatedAt = Instant.now(),
    isPendingSync = false,
)

private class FakeConnectivityObserver(
    online: Boolean = true,
) : ConnectivityObserver {
    override val isOnline: StateFlow<Boolean> = MutableStateFlow(online).asStateFlow()
}

private val stubJson = Json { ignoreUnknownKeys = true }

@Suppress("UNCHECKED_CAST")
private fun stubPagoDao(): com.propmanager.core.database.dao.PagoDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.PagoDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.PagoDao::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.database.dao.PagoDao

@Suppress("UNCHECKED_CAST")
private fun stubSyncQueueDao(): com.propmanager.core.database.dao.SyncQueueDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.SyncQueueDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.SyncQueueDao::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.database.dao.SyncQueueDao

@Suppress("UNCHECKED_CAST")
private fun stubPagosApiService(): com.propmanager.core.network.api.PagosApiService =
    Proxy.newProxyInstance(
        com.propmanager.core.network.api.PagosApiService::class.java.classLoader,
        arrayOf(com.propmanager.core.network.api.PagosApiService::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.network.api.PagosApiService

@Suppress("UNCHECKED_CAST")
private fun stubContratoDao(): com.propmanager.core.database.dao.ContratoDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.ContratoDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.ContratoDao::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.database.dao.ContratoDao

@Suppress("UNCHECKED_CAST")
private fun stubContratosApiService(): com.propmanager.core.network.api.ContratosApiService =
    Proxy.newProxyInstance(
        com.propmanager.core.network.api.ContratosApiService::class.java.classLoader,
        arrayOf(com.propmanager.core.network.api.ContratosApiService::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.network.api.ContratosApiService

private class FakePagosRepository(
    private val initialData: List<Pago> = emptyList(),
    private val createError: Throwable? = null,
    private val updateError: Throwable? = null,
) : PagosRepository(
        dao = stubPagoDao(),
        syncQueueDao = stubSyncQueueDao(),
        apiService = stubPagosApiService(),
        json = stubJson,
    ) {
    var createCallCount = 0
        private set
    var updateCallCount = 0
        private set
    var deleteCallCount = 0
        private set

    private val store = MutableStateFlow(initialData)

    override fun observeAll(): Flow<List<Pago>> = store

    override fun observeFiltered(
        contratoId: String?,
        estado: String?,
        fechaDesde: String?,
        fechaHasta: String?,
    ): Flow<List<Pago>> =
        store.map { list ->
            list.filter { p ->
                (contratoId == null || p.contratoId == contratoId) &&
                    (estado == null || p.estado == estado) &&
                    (fechaDesde == null || p.fechaVencimiento.toString() >= fechaDesde) &&
                    (fechaHasta == null || p.fechaVencimiento.toString() <= fechaHasta)
            }
        }

    override suspend fun create(request: CreatePagoRequest): Result<Pago> {
        createCallCount++
        if (createError != null) return Result.failure(createError)
        val p = Pago(
            id = "new-$createCallCount",
            contratoId = request.contratoId,
            monto = request.monto.toBigDecimal(),
            moneda = request.moneda ?: "DOP",
            fechaPago = request.fechaPago?.let { LocalDate.parse(it) },
            fechaVencimiento = LocalDate.parse(request.fechaVencimiento),
            metodoPago = request.metodoPago,
            estado = "pendiente",
            notas = request.notas,
            createdAt = Instant.now(),
            updatedAt = Instant.now(),
            isPendingSync = true,
        )
        store.value = store.value + p
        return Result.success(p)
    }

    override suspend fun update(
        id: String,
        request: UpdatePagoRequest,
    ): Result<Unit> {
        updateCallCount++
        if (updateError != null) return Result.failure(updateError)
        return Result.success(Unit)
    }

    override suspend fun delete(id: String): Result<Unit> {
        deleteCallCount++
        store.value = store.value.filter { it.id != id }
        return Result.success(Unit)
    }
}

private class FakeContratosRepository(
    private val initialData: List<Contrato> = emptyList(),
) : ContratosRepository(
        dao = stubContratoDao(),
        syncQueueDao = stubSyncQueueDao(),
        apiService = stubContratosApiService(),
        json = stubJson,
    ) {
    override fun observeAll(): Flow<List<Contrato>> = MutableStateFlow(initialData)
}
