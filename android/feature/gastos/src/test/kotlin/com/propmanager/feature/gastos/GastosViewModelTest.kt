package com.propmanager.feature.gastos

import com.propmanager.core.data.repository.GastosRepository
import com.propmanager.core.data.repository.PropiedadesRepository
import com.propmanager.core.model.Gasto
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.dto.CreateGastoRequest
import com.propmanager.core.model.dto.CreatePropiedadRequest
import com.propmanager.core.model.dto.UpdateGastoRequest
import com.propmanager.core.model.dto.UpdatePropiedadRequest
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
 * Unit tests for GastosViewModel.
 *
 * Validates: Requirements 7.1, 7.2, 7.7
 */
@OptIn(ExperimentalCoroutinesApi::class)
class GastosViewModelTest :
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
                "emits Success with gastos from repository" {
                    val repo = FakeGastosRepository(
                        initialData = listOf(
                            sampleGasto("1", categoria = "mantenimiento"),
                            sampleGasto("2", categoria = "servicios"),
                        ),
                    )
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        val state = vm.gastos.value
                        state.shouldBeInstanceOf<GastosUiState.Success>()
                        state.gastos shouldHaveSize 2
                    }
                }

                "emits Success with empty list when no gastos" {
                    val repo = FakeGastosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        val state = vm.gastos.value
                        state.shouldBeInstanceOf<GastosUiState.Success>()
                        state.gastos shouldHaveSize 0
                    }
                }
            }

        "filter application (Req 7.2)" -
            {
                "updateFilter filters by propiedadId" {
                    val repo = FakeGastosRepository(
                        initialData = listOf(
                            sampleGasto("1", propiedadId = "p1"),
                            sampleGasto("2", propiedadId = "p2"),
                            sampleGasto("3", propiedadId = "p1"),
                        ),
                    )
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(propiedadId = "p1")
                        advanceUntilIdle()

                        vm.filters.value.propiedadId shouldBe "p1"
                        val state = vm.gastos.value
                        state.shouldBeInstanceOf<GastosUiState.Success>()
                        state.gastos shouldHaveSize 2
                        state.gastos.all { it.propiedadId == "p1" } shouldBe true
                    }
                }

                "updateFilter filters by categoria" {
                    val repo = FakeGastosRepository(
                        initialData = listOf(
                            sampleGasto("1", categoria = "mantenimiento"),
                            sampleGasto("2", categoria = "servicios"),
                            sampleGasto("3", categoria = "mantenimiento"),
                        ),
                    )
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(categoria = "mantenimiento")
                        advanceUntilIdle()

                        val state = vm.gastos.value
                        state.shouldBeInstanceOf<GastosUiState.Success>()
                        state.gastos shouldHaveSize 2
                        state.gastos.all { it.categoria == "mantenimiento" } shouldBe true
                    }
                }

                "updateFilter filters by estado" {
                    val repo = FakeGastosRepository(
                        initialData = listOf(
                            sampleGasto("1", estado = "pendiente"),
                            sampleGasto("2", estado = "pagado"),
                            sampleGasto("3", estado = "pendiente"),
                        ),
                    )
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(estado = "pendiente")
                        advanceUntilIdle()

                        val state = vm.gastos.value
                        state.shouldBeInstanceOf<GastosUiState.Success>()
                        state.gastos shouldHaveSize 2
                        state.gastos.all { it.estado == "pendiente" } shouldBe true
                    }
                }

                "clearFilters resets all filters and shows all gastos" {
                    val repo = FakeGastosRepository(
                        initialData = listOf(
                            sampleGasto("1", propiedadId = "p1"),
                            sampleGasto("2", propiedadId = "p2"),
                        ),
                    )
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(propiedadId = "p1")
                        advanceUntilIdle()

                        vm.clearFilters()
                        advanceUntilIdle()

                        vm.filters.value shouldBe GastosFilterState()
                        val state = vm.gastos.value
                        state.shouldBeInstanceOf<GastosUiState.Success>()
                        state.gastos shouldHaveSize 2
                    }
                }
            }

        "validation errors (Req 7.7)" -
            {
                "save with blank required fields shows validation errors and does not call repository" {
                    val repo = FakeGastosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        val errors = vm.formState.value.errors
                        errors shouldContainKey "propiedadId"
                        errors shouldContainKey "categoria"
                        errors shouldContainKey "descripcion"
                        errors shouldContainKey "monto"
                        errors shouldContainKey "fechaGasto"
                        errors["propiedadId"]!!.shouldNotBeBlank()
                        repo.createCallCount shouldBe 0
                    }
                }

                "onFieldChange clears error for that field" {
                    val repo = FakeGastosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "propiedadId"
                        vm.onFieldChange("propiedadId", "p1")
                        vm.formState.value.errors.containsKey("propiedadId") shouldBe false
                    }
                }

                "onFechaGastoChange clears fechaGasto error" {
                    val repo = FakeGastosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "fechaGasto"
                        vm.onFechaGastoChange(LocalDate.of(2025, 6, 1))
                        vm.formState.value.errors.containsKey("fechaGasto") shouldBe false
                    }
                }
            }

        "CRUD state transitions" -
            {
                "create with valid data succeeds and calls onSuccess" {
                    val repo = FakeGastosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("propiedadId", "p1")
                        vm.onFieldChange("categoria", "mantenimiento")
                        vm.onFieldChange("descripcion", "Reparación de techo")
                        vm.onFieldChange("monto", "5000.00")
                        vm.onFechaGastoChange(LocalDate.of(2025, 6, 1))

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
                    val existing = sampleGasto("1")
                    val repo = FakeGastosRepository(initialData = listOf(existing))
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
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

                "create failure sets general error on form" {
                    val repo = FakeGastosRepository(createError = RuntimeException("DB error"))
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("propiedadId", "p1")
                        vm.onFieldChange("categoria", "mantenimiento")
                        vm.onFieldChange("descripcion", "Reparación")
                        vm.onFieldChange("monto", "5000")
                        vm.onFechaGastoChange(LocalDate.of(2025, 6, 1))

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe false
                        vm.formState.value.errors shouldContainKey "general"
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }

                "delete flow sets target, confirms, and clears" {
                    val target = sampleGasto("1")
                    val repo = FakeGastosRepository(initialData = listOf(target))
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
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
                    val target = sampleGasto("1")
                    val repo = FakeGastosRepository(initialData = listOf(target))
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.requestDelete(target)
                        vm.deleteTarget.value.shouldNotBeNull()

                        vm.dismissDelete()
                        vm.deleteTarget.value.shouldBeNull()
                        repo.deleteCallCount shouldBe 0
                    }
                }
            }

        "OCR data pre-fill" -
            {
                "prefillFromOcr sets monto, fecha, proveedor, and numeroFactura on form" {
                    val repo = FakeGastosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.prefillFromOcr(
                            monto = "3500.00",
                            fecha = LocalDate.of(2025, 5, 15),
                            proveedor = "Ferretería Central",
                            numeroFactura = "F-001234",
                        )

                        vm.formState.value.monto shouldBe "3500.00"
                        vm.formState.value.fechaGasto shouldBe LocalDate.of(2025, 5, 15)
                        vm.formState.value.proveedor shouldBe "Ferretería Central"
                        vm.formState.value.numeroFactura shouldBe "F-001234"
                    }
                }

                "prefillFromOcr with null values preserves existing form data" {
                    val repo = FakeGastosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("monto", "1000")
                        vm.onFieldChange("proveedor", "Existing")
                        vm.prefillFromOcr(monto = null, fecha = LocalDate.of(2025, 3, 1), proveedor = null, numeroFactura = "F-999")

                        vm.formState.value.monto shouldBe "1000"
                        vm.formState.value.proveedor shouldBe "Existing"
                        vm.formState.value.fechaGasto shouldBe LocalDate.of(2025, 3, 1)
                        vm.formState.value.numeroFactura shouldBe "F-999"
                    }
                }

                "prefillFromOcr data can be saved successfully" {
                    val repo = FakeGastosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("propiedadId", "p1")
                        vm.onFieldChange("categoria", "servicios")
                        vm.onFieldChange("descripcion", "Factura de electricidad")
                        vm.prefillFromOcr(
                            monto = "2500.00",
                            fecha = LocalDate.of(2025, 4, 10),
                            proveedor = "EDENORTE",
                            numeroFactura = "E-5678",
                        )

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe true
                        repo.createCallCount shouldBe 1
                    }
                }
            }

        "form initialization" -
            {
                "initCreateForm resets form to defaults" {
                    val repo = FakeGastosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.onFieldChange("propiedadId", "p1")
                        vm.initCreateForm()

                        vm.formState.value.propiedadId shouldBe ""
                        vm.formState.value.categoria shouldBe ""
                        vm.formState.value.errors.shouldBeEmpty()
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }

                "initEditForm populates form from gasto" {
                    val gasto = sampleGasto(
                        "1",
                        propiedadId = "p1",
                        categoria = "servicios",
                        descripcion = "Agua",
                        monto = BigDecimal("2500.00"),
                        proveedor = "CORAASAN",
                        numeroFactura = "W-123",
                    )
                    val repo = FakeGastosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(gastosRepo = repo)
                        advanceUntilIdle()

                        vm.initEditForm(gasto)

                        vm.formState.value.propiedadId shouldBe "p1"
                        vm.formState.value.categoria shouldBe "servicios"
                        vm.formState.value.descripcion shouldBe "Agua"
                        vm.formState.value.monto shouldBe "2500.00"
                        vm.formState.value.proveedor shouldBe "CORAASAN"
                        vm.formState.value.numeroFactura shouldBe "W-123"
                        vm.formState.value.fechaGasto shouldBe gasto.fechaGasto
                    }
                }
            }

        "clearSuccessMessage resets success message" {
            val repo = FakeGastosRepository(initialData = listOf(sampleGasto("1")))
            runTest(testDispatcher) {
                val vm = createViewModel(gastosRepo = repo)
                advanceUntilIdle()

                vm.requestDelete(sampleGasto("1"))
                vm.confirmDelete()
                advanceUntilIdle()

                vm.successMessage.value.shouldNotBeNull()
                vm.clearSuccessMessage()
                vm.successMessage.value.shouldBeNull()
            }
        }
    })

private fun createViewModel(
    gastosRepo: FakeGastosRepository = FakeGastosRepository(),
    propRepo: FakePropiedadesRepository = FakePropiedadesRepository(),
    connectivity: FakeConnectivityObserver = FakeConnectivityObserver(),
): GastosViewModel = GastosViewModel(gastosRepo, propRepo, connectivity)

private fun sampleGasto(
    id: String,
    propiedadId: String = "p1",
    categoria: String = "mantenimiento",
    descripcion: String = "Reparación general",
    monto: BigDecimal = BigDecimal("5000.00"),
    moneda: String = "DOP",
    fechaGasto: LocalDate = LocalDate.of(2025, 6, 1),
    estado: String = "pendiente",
    proveedor: String? = null,
    numeroFactura: String? = null,
) = Gasto(
    id = id,
    propiedadId = propiedadId,
    unidadId = null,
    categoria = categoria,
    descripcion = descripcion,
    monto = monto,
    moneda = moneda,
    fechaGasto = fechaGasto,
    estado = estado,
    proveedor = proveedor,
    numeroFactura = numeroFactura,
    notas = null,
    createdAt = Instant.now(),
    updatedAt = Instant.now(),
    isPendingSync = false,
)

private fun samplePropiedad(
    id: String,
    titulo: String = "Propiedad $id",
) = Propiedad(
    id = id,
    titulo = titulo,
    descripcion = null,
    direccion = "Calle $id",
    ciudad = "Santiago",
    provincia = "Santiago",
    tipoPropiedad = "apartamento",
    habitaciones = 2,
    banos = 1,
    areaM2 = BigDecimal("80"),
    precio = BigDecimal("15000.00"),
    moneda = "DOP",
    estado = "disponible",
    imagenes = emptyList(),
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
private fun stubGastoDao(): com.propmanager.core.database.dao.GastoDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.GastoDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.GastoDao::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.database.dao.GastoDao

@Suppress("UNCHECKED_CAST")
private fun stubSyncQueueDao(): com.propmanager.core.database.dao.SyncQueueDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.SyncQueueDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.SyncQueueDao::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.database.dao.SyncQueueDao

@Suppress("UNCHECKED_CAST")
private fun stubGastosApiService(): com.propmanager.core.network.api.GastosApiService =
    Proxy.newProxyInstance(
        com.propmanager.core.network.api.GastosApiService::class.java.classLoader,
        arrayOf(com.propmanager.core.network.api.GastosApiService::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.network.api.GastosApiService

@Suppress("UNCHECKED_CAST")
private fun stubPropiedadDao(): com.propmanager.core.database.dao.PropiedadDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.PropiedadDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.PropiedadDao::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.database.dao.PropiedadDao

@Suppress("UNCHECKED_CAST")
private fun stubPropiedadesApiService(): com.propmanager.core.network.api.PropiedadesApiService =
    Proxy.newProxyInstance(
        com.propmanager.core.network.api.PropiedadesApiService::class.java.classLoader,
        arrayOf(com.propmanager.core.network.api.PropiedadesApiService::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.network.api.PropiedadesApiService

private class FakeGastosRepository(
    private val initialData: List<Gasto> = emptyList(),
    private val createError: Throwable? = null,
    private val updateError: Throwable? = null,
) : GastosRepository(
        dao = stubGastoDao(),
        syncQueueDao = stubSyncQueueDao(),
        apiService = stubGastosApiService(),
        json = stubJson,
    ) {
    var createCallCount = 0
        private set
    var updateCallCount = 0
        private set
    var deleteCallCount = 0
        private set

    private val store = MutableStateFlow(initialData)

    override fun observeAll(): Flow<List<Gasto>> = store

    override fun observeFiltered(
        propiedadId: String?,
        categoria: String?,
        estado: String?,
        fechaDesde: String?,
        fechaHasta: String?,
    ): Flow<List<Gasto>> =
        store.map { list ->
            list.filter { g ->
                (propiedadId == null || g.propiedadId == propiedadId) &&
                    (categoria == null || g.categoria == categoria) &&
                    (estado == null || g.estado == estado) &&
                    (fechaDesde == null || g.fechaGasto.toString() >= fechaDesde) &&
                    (fechaHasta == null || g.fechaGasto.toString() <= fechaHasta)
            }
        }

    override suspend fun create(request: CreateGastoRequest): Result<Gasto> {
        createCallCount++
        if (createError != null) return Result.failure(createError)
        val g = Gasto(
            id = "new-$createCallCount",
            propiedadId = request.propiedadId,
            unidadId = request.unidadId,
            categoria = request.categoria,
            descripcion = request.descripcion,
            monto = request.monto.toBigDecimal(),
            moneda = request.moneda,
            fechaGasto = LocalDate.parse(request.fechaGasto),
            estado = "pendiente",
            proveedor = request.proveedor,
            numeroFactura = request.numeroFactura,
            notas = request.notas,
            createdAt = Instant.now(),
            updatedAt = Instant.now(),
            isPendingSync = true,
        )
        store.value = store.value + g
        return Result.success(g)
    }

    override suspend fun update(
        id: String,
        request: UpdateGastoRequest,
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

private class FakePropiedadesRepository(
    private val initialData: List<Propiedad> = emptyList(),
) : PropiedadesRepository(
        dao = stubPropiedadDao(),
        syncQueueDao = stubSyncQueueDao(),
        apiService = stubPropiedadesApiService(),
        json = stubJson,
    ) {
    override fun observeAll(): Flow<List<Propiedad>> = MutableStateFlow(initialData)
}
