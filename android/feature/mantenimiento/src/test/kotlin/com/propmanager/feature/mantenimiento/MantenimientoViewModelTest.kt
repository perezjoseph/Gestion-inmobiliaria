package com.propmanager.feature.mantenimiento

import com.propmanager.core.data.repository.MantenimientoRepository
import com.propmanager.core.data.repository.PropiedadesRepository
import com.propmanager.core.model.NotaMantenimiento
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.SolicitudMantenimiento
import com.propmanager.core.model.dto.CreateNotaRequest
import com.propmanager.core.model.dto.CreateSolicitudRequest
import com.propmanager.core.model.dto.UpdateEstadoRequest
import com.propmanager.core.model.dto.UpdateSolicitudRequest
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

/**
 * Unit tests for MantenimientoViewModel.
 *
 * Validates: Requirements 8.1, 8.2, 8.8
 */
@OptIn(ExperimentalCoroutinesApi::class)
class MantenimientoViewModelTest :
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
                "emits Success with solicitudes from repository" {
                    val repo = FakeMantenimientoRepository(
                        initialData = listOf(
                            sampleSolicitud("1", estado = "pendiente"),
                            sampleSolicitud("2", estado = "en_progreso"),
                        ),
                    )
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        val state = vm.solicitudes.value
                        state.shouldBeInstanceOf<MantenimientoUiState.Success>()
                        state.solicitudes shouldHaveSize 2
                    }
                }

                "emits Success with empty list when no solicitudes" {
                    val repo = FakeMantenimientoRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        val state = vm.solicitudes.value
                        state.shouldBeInstanceOf<MantenimientoUiState.Success>()
                        state.solicitudes shouldHaveSize 0
                    }
                }
            }

        "filter application (Req 8.2)" -
            {
                "updateFilter filters by estado" {
                    val repo = FakeMantenimientoRepository(
                        initialData = listOf(
                            sampleSolicitud("1", estado = "pendiente"),
                            sampleSolicitud("2", estado = "en_progreso"),
                            sampleSolicitud("3", estado = "pendiente"),
                        ),
                    )
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(estado = "pendiente")
                        advanceUntilIdle()

                        vm.filters.value.estado shouldBe "pendiente"
                        val state = vm.solicitudes.value
                        state.shouldBeInstanceOf<MantenimientoUiState.Success>()
                        state.solicitudes shouldHaveSize 2
                        state.solicitudes.all { it.estado == "pendiente" } shouldBe true
                    }
                }

                "updateFilter filters by prioridad" {
                    val repo = FakeMantenimientoRepository(
                        initialData = listOf(
                            sampleSolicitud("1", prioridad = "alta"),
                            sampleSolicitud("2", prioridad = "baja"),
                            sampleSolicitud("3", prioridad = "alta"),
                        ),
                    )
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(prioridad = "alta")
                        advanceUntilIdle()

                        val state = vm.solicitudes.value
                        state.shouldBeInstanceOf<MantenimientoUiState.Success>()
                        state.solicitudes shouldHaveSize 2
                        state.solicitudes.all { it.prioridad == "alta" } shouldBe true
                    }
                }

                "updateFilter filters by propiedadId" {
                    val repo = FakeMantenimientoRepository(
                        initialData = listOf(
                            sampleSolicitud("1", propiedadId = "p1"),
                            sampleSolicitud("2", propiedadId = "p2"),
                            sampleSolicitud("3", propiedadId = "p1"),
                        ),
                    )
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(propiedadId = "p1")
                        advanceUntilIdle()

                        val state = vm.solicitudes.value
                        state.shouldBeInstanceOf<MantenimientoUiState.Success>()
                        state.solicitudes shouldHaveSize 2
                        state.solicitudes.all { it.propiedadId == "p1" } shouldBe true
                    }
                }

                "clearFilters resets all filters and shows all solicitudes" {
                    val repo = FakeMantenimientoRepository(
                        initialData = listOf(
                            sampleSolicitud("1", propiedadId = "p1"),
                            sampleSolicitud("2", propiedadId = "p2"),
                        ),
                    )
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.updateFilter(propiedadId = "p1")
                        advanceUntilIdle()

                        vm.clearFilters()
                        advanceUntilIdle()

                        vm.filters.value shouldBe MantenimientoFilterState()
                        val state = vm.solicitudes.value
                        state.shouldBeInstanceOf<MantenimientoUiState.Success>()
                        state.solicitudes shouldHaveSize 2
                    }
                }
            }

        "validation errors (Req 8.8)" -
            {
                "save with blank required fields shows validation errors and does not call repository" {
                    val repo = FakeMantenimientoRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        val errors = vm.formState.value.errors
                        errors shouldContainKey "propiedadId"
                        errors shouldContainKey "titulo"
                        errors["propiedadId"]!!.shouldNotBeBlank()
                        errors["titulo"]!!.shouldNotBeBlank()
                        repo.createCallCount shouldBe 0
                    }
                }

                "onFieldChange clears error for that field" {
                    val repo = FakeMantenimientoRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "propiedadId"
                        vm.onFieldChange("propiedadId", "p1")
                        vm.formState.value.errors.containsKey("propiedadId") shouldBe false
                    }
                }
            }

        "CRUD state transitions" -
            {
                "create with valid data succeeds and calls onSuccess" {
                    val repo = FakeMantenimientoRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("propiedadId", "p1")
                        vm.onFieldChange("titulo", "Fuga de agua")

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
                    val existing = sampleSolicitud("1")
                    val repo = FakeMantenimientoRepository(initialData = listOf(existing))
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.initEditForm(existing)
                        vm.onFieldChange("titulo", "Fuga de agua actualizada")

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe true
                        vm.successMessage.value shouldBe "Actualizado correctamente"
                        repo.updateCallCount shouldBe 1
                    }
                }

                "create failure sets general error on form" {
                    val repo = FakeMantenimientoRepository(createError = RuntimeException("DB error"))
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("propiedadId", "p1")
                        vm.onFieldChange("titulo", "Reparación")

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe false
                        vm.formState.value.errors shouldContainKey "general"
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }

                "delete flow sets target, confirms, and clears" {
                    val target = sampleSolicitud("1")
                    val repo = FakeMantenimientoRepository(initialData = listOf(target))
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
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
                    val target = sampleSolicitud("1")
                    val repo = FakeMantenimientoRepository(initialData = listOf(target))
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.requestDelete(target)
                        vm.deleteTarget.value.shouldNotBeNull()

                        vm.dismissDelete()
                        vm.deleteTarget.value.shouldBeNull()
                        repo.deleteCallCount shouldBe 0
                    }
                }
            }

        "status change" -
            {
                "changeEstado calls repository and shows success message" {
                    val solicitud = sampleSolicitud("1", estado = "pendiente")
                    val repo = FakeMantenimientoRepository(initialData = listOf(solicitud))
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.loadDetail("1")
                        advanceUntilIdle()

                        vm.changeEstado("en_progreso")
                        advanceUntilIdle()

                        vm.successMessage.value shouldBe "Estado actualizado"
                        repo.updateEstadoCallCount shouldBe 1
                        repo.lastEstadoRequest?.estado shouldBe "en_progreso"
                        vm.showEstadoDialog.value shouldBe false
                    }
                }

                "showEstadoChange and dismissEstadoChange toggle dialog" {
                    val repo = FakeMantenimientoRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.showEstadoDialog.value shouldBe false
                        vm.showEstadoChange()
                        vm.showEstadoDialog.value shouldBe true
                        vm.dismissEstadoChange()
                        vm.showEstadoDialog.value shouldBe false
                    }
                }
            }

        "add nota" -
            {
                "addNota with non-blank content calls repository and clears input" {
                    val solicitud = sampleSolicitud("1")
                    val repo = FakeMantenimientoRepository(initialData = listOf(solicitud))
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.loadDetail("1")
                        advanceUntilIdle()

                        vm.onNotaInputChange("Se revisó la tubería")
                        vm.notaInput.value shouldBe "Se revisó la tubería"

                        vm.addNota()
                        advanceUntilIdle()

                        vm.notaInput.value shouldBe ""
                        vm.successMessage.value shouldBe "Nota agregada"
                        repo.addNotaCallCount shouldBe 1
                    }
                }

                "addNota with blank content does not call repository" {
                    val solicitud = sampleSolicitud("1")
                    val repo = FakeMantenimientoRepository(initialData = listOf(solicitud))
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.loadDetail("1")
                        advanceUntilIdle()

                        vm.onNotaInputChange("   ")
                        vm.addNota()
                        advanceUntilIdle()

                        repo.addNotaCallCount shouldBe 0
                    }
                }
            }

        "form initialization" -
            {
                "initCreateForm resets form to defaults" {
                    val repo = FakeMantenimientoRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.onFieldChange("propiedadId", "p1")
                        vm.initCreateForm()

                        vm.formState.value.propiedadId shouldBe ""
                        vm.formState.value.titulo shouldBe ""
                        vm.formState.value.errors.shouldBeEmpty()
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }

                "initEditForm populates form from solicitud" {
                    val solicitud = sampleSolicitud(
                        "1",
                        propiedadId = "p1",
                        titulo = "Fuga de agua",
                        descripcion = "En el baño principal",
                        prioridad = "alta",
                        nombreProveedor = "Plomero Juan",
                        costoMonto = BigDecimal("3500.00"),
                    )
                    val repo = FakeMantenimientoRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(mantenimientoRepo = repo)
                        advanceUntilIdle()

                        vm.initEditForm(solicitud)

                        vm.formState.value.propiedadId shouldBe "p1"
                        vm.formState.value.titulo shouldBe "Fuga de agua"
                        vm.formState.value.descripcion shouldBe "En el baño principal"
                        vm.formState.value.prioridad shouldBe "alta"
                        vm.formState.value.nombreProveedor shouldBe "Plomero Juan"
                        vm.formState.value.costoMonto shouldBe "3500.00"
                    }
                }
            }

        "clearSuccessMessage resets success message" {
            val repo = FakeMantenimientoRepository(initialData = listOf(sampleSolicitud("1")))
            runTest(testDispatcher) {
                val vm = createViewModel(mantenimientoRepo = repo)
                advanceUntilIdle()

                vm.requestDelete(sampleSolicitud("1"))
                vm.confirmDelete()
                advanceUntilIdle()

                vm.successMessage.value.shouldNotBeNull()
                vm.clearSuccessMessage()
                vm.successMessage.value.shouldBeNull()
            }
        }
    })

private fun createViewModel(
    mantenimientoRepo: FakeMantenimientoRepository = FakeMantenimientoRepository(),
    propRepo: FakePropiedadesRepository = FakePropiedadesRepository(),
    connectivity: FakeConnectivityObserver = FakeConnectivityObserver(),
): MantenimientoViewModel = MantenimientoViewModel(mantenimientoRepo, propRepo, connectivity)

private fun sampleSolicitud(
    id: String,
    propiedadId: String = "p1",
    titulo: String = "Reparación general",
    descripcion: String? = null,
    estado: String = "pendiente",
    prioridad: String = "media",
    nombreProveedor: String? = null,
    telefonoProveedor: String? = null,
    emailProveedor: String? = null,
    costoMonto: BigDecimal? = null,
    costoMoneda: String? = null,
) = SolicitudMantenimiento(
    id = id,
    propiedadId = propiedadId,
    unidadId = null,
    inquilinoId = null,
    titulo = titulo,
    descripcion = descripcion,
    estado = estado,
    prioridad = prioridad,
    nombreProveedor = nombreProveedor,
    telefonoProveedor = telefonoProveedor,
    emailProveedor = emailProveedor,
    costoMonto = costoMonto,
    costoMoneda = costoMoneda,
    fechaInicio = null,
    fechaFin = null,
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
private fun stubSolicitudDao(): com.propmanager.core.database.dao.SolicitudMantenimientoDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.SolicitudMantenimientoDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.SolicitudMantenimientoDao::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.database.dao.SolicitudMantenimientoDao

@Suppress("UNCHECKED_CAST")
private fun stubNotaDao(): com.propmanager.core.database.dao.NotaMantenimientoDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.NotaMantenimientoDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.NotaMantenimientoDao::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.database.dao.NotaMantenimientoDao

@Suppress("UNCHECKED_CAST")
private fun stubSyncQueueDao(): com.propmanager.core.database.dao.SyncQueueDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.SyncQueueDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.SyncQueueDao::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.database.dao.SyncQueueDao

@Suppress("UNCHECKED_CAST")
private fun stubMantenimientoApiService(): com.propmanager.core.network.api.MantenimientoApiService =
    Proxy.newProxyInstance(
        com.propmanager.core.network.api.MantenimientoApiService::class.java.classLoader,
        arrayOf(com.propmanager.core.network.api.MantenimientoApiService::class.java),
    ) { _, _, _ -> error("stub") } as com.propmanager.core.network.api.MantenimientoApiService

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

private class FakeMantenimientoRepository(
    private val initialData: List<SolicitudMantenimiento> = emptyList(),
    private val createError: Throwable? = null,
    private val updateError: Throwable? = null,
) : MantenimientoRepository(
        solicitudDao = stubSolicitudDao(),
        notaDao = stubNotaDao(),
        syncQueueDao = stubSyncQueueDao(),
        apiService = stubMantenimientoApiService(),
        json = stubJson,
    ) {
    var createCallCount = 0
        private set
    var updateCallCount = 0
        private set
    var deleteCallCount = 0
        private set
    var updateEstadoCallCount = 0
        private set
    var addNotaCallCount = 0
        private set
    var lastEstadoRequest: UpdateEstadoRequest? = null
        private set

    private val store = MutableStateFlow(initialData)
    private val notasStore = MutableStateFlow<List<NotaMantenimiento>>(emptyList())

    override fun observeAll(): Flow<List<SolicitudMantenimiento>> = store

    override fun observeFiltered(
        estado: String?,
        prioridad: String?,
        propiedadId: String?,
    ): Flow<List<SolicitudMantenimiento>> =
        store.map { list ->
            list.filter { s ->
                (estado == null || s.estado == estado) &&
                    (prioridad == null || s.prioridad == prioridad) &&
                    (propiedadId == null || s.propiedadId == propiedadId)
            }
        }

    override fun observeById(id: String): Flow<SolicitudMantenimiento?> =
        store.map { list -> list.find { it.id == id } }

    override fun observeNotas(solicitudId: String): Flow<List<NotaMantenimiento>> =
        notasStore.map { list -> list.filter { it.solicitudId == solicitudId } }

    override suspend fun create(request: CreateSolicitudRequest): Result<SolicitudMantenimiento> {
        createCallCount++
        if (createError != null) return Result.failure(createError)
        val s = SolicitudMantenimiento(
            id = "new-$createCallCount",
            propiedadId = request.propiedadId,
            unidadId = request.unidadId,
            inquilinoId = request.inquilinoId,
            titulo = request.titulo,
            descripcion = request.descripcion,
            estado = "pendiente",
            prioridad = request.prioridad ?: "media",
            nombreProveedor = request.nombreProveedor,
            telefonoProveedor = request.telefonoProveedor,
            emailProveedor = request.emailProveedor,
            costoMonto = request.costoMonto?.toBigDecimalOrNull(),
            costoMoneda = request.costoMoneda,
            fechaInicio = null,
            fechaFin = null,
            createdAt = Instant.now(),
            updatedAt = Instant.now(),
            isPendingSync = true,
        )
        store.value = store.value + s
        return Result.success(s)
    }

    override suspend fun update(
        id: String,
        request: UpdateSolicitudRequest,
    ): Result<Unit> {
        updateCallCount++
        if (updateError != null) return Result.failure(updateError)
        return Result.success(Unit)
    }

    override suspend fun updateEstado(
        id: String,
        request: UpdateEstadoRequest,
    ): Result<Unit> {
        updateEstadoCallCount++
        lastEstadoRequest = request
        return Result.success(Unit)
    }

    override suspend fun addNota(
        solicitudId: String,
        request: CreateNotaRequest,
    ): Result<NotaMantenimiento> {
        addNotaCallCount++
        val nota = NotaMantenimiento(
            id = "nota-$addNotaCallCount",
            solicitudId = solicitudId,
            autorId = "",
            contenido = request.contenido,
            createdAt = Instant.now(),
        )
        notasStore.value = notasStore.value + nota
        return Result.success(nota)
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
