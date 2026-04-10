package com.propmanager.feature.contratos

import com.propmanager.core.data.repository.ContratosRepository
import com.propmanager.core.data.repository.InquilinosRepository
import com.propmanager.core.data.repository.PropiedadesRepository
import com.propmanager.core.model.Contrato
import com.propmanager.core.model.Inquilino
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.dto.CreateContratoRequest
import com.propmanager.core.model.dto.RenovarContratoRequest
import com.propmanager.core.model.dto.TerminarContratoRequest
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
import java.lang.reflect.Proxy
import java.math.BigDecimal
import java.time.Instant
import java.time.LocalDate
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

/**
 * Unit tests for ContratosViewModel.
 *
 * Validates: Requirements 5.7, 5.8, 5.6
 */
@OptIn(ExperimentalCoroutinesApi::class)
class ContratosViewModelTest :
    FreeSpec({
        val testDispatcher = StandardTestDispatcher()

        beforeEach { Dispatchers.setMain(testDispatcher) }

        afterEach { Dispatchers.resetMain() }

        "list state" -
            {
                "emits Success with contratos and resolved names" {
                    val propiedades = listOf(samplePropiedad("p1", titulo = "Casa Centro"))
                    val inquilinos =
                        listOf(sampleInquilino("i1", nombre = "Juan", apellido = "Perez"))
                    val contratos =
                        listOf(sampleContrato("c1", propiedadId = "p1", inquilinoId = "i1"))
                    val contratosRepo = FakeContratosRepository(initialData = contratos)
                    val propRepo = FakePropiedadesRepository(initialData = propiedades)
                    val inqRepo = FakeInquilinosRepository(initialData = inquilinos)

                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo, propRepo, inqRepo)
                        advanceUntilIdle()

                        val state = vm.contratos.value
                        state.shouldBeInstanceOf<ContratosUiState.Success>()
                        state.contratos shouldHaveSize 1
                        state.contratos.first().propiedadTitulo shouldBe "Casa Centro"
                        state.contratos.first().inquilinoNombre shouldBe "Juan Perez"
                    }
                }

                "emits Success with empty list when no contratos" {
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        val state = vm.contratos.value
                        state.shouldBeInstanceOf<ContratosUiState.Success>()
                        state.contratos shouldHaveSize 0
                    }
                }
            }

        "date validation (Req 5.8)" -
            {
                "save with fecha_fin before fecha_inicio shows validation error" {
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFormFieldChange("propiedadId", "p1")
                        vm.onFormFieldChange("inquilinoId", "i1")
                        vm.onFechaInicioChange(LocalDate.of(2025, 6, 1))
                        vm.onFechaFinChange(LocalDate.of(2025, 5, 1))
                        vm.onFormFieldChange("montoMensual", "15000")

                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "fechaFin"
                        contratosRepo.createCallCount shouldBe 0
                    }
                }

                "save with fecha_fin equal to fecha_inicio shows validation error" {
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFormFieldChange("propiedadId", "p1")
                        vm.onFormFieldChange("inquilinoId", "i1")
                        val sameDate = LocalDate.of(2025, 6, 1)
                        vm.onFechaInicioChange(sameDate)
                        vm.onFechaFinChange(sameDate)
                        vm.onFormFieldChange("montoMensual", "15000")

                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "fechaFin"
                        contratosRepo.createCallCount shouldBe 0
                    }
                }

                "save with fecha_fin after fecha_inicio succeeds" {
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFormFieldChange("propiedadId", "p1")
                        vm.onFormFieldChange("inquilinoId", "i1")
                        vm.onFechaInicioChange(LocalDate.of(2025, 1, 1))
                        vm.onFechaFinChange(LocalDate.of(2025, 12, 31))
                        vm.onFormFieldChange("montoMensual", "15000")

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe true
                        vm.formState.value.errors.shouldBeEmpty()
                        contratosRepo.createCallCount shouldBe 1
                    }
                }
            }

        "validation errors (Req 5.7)" -
            {
                "save with blank required fields shows validation errors" {
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        val errors = vm.formState.value.errors
                        errors shouldContainKey "propiedadId"
                        errors shouldContainKey "inquilinoId"
                        errors shouldContainKey "fechaInicio"
                        errors shouldContainKey "fechaFin"
                        errors shouldContainKey "montoMensual"
                        errors["propiedadId"]!!.shouldNotBeBlank()
                        contratosRepo.createCallCount shouldBe 0
                    }
                }

                "onFormFieldChange clears error for that field" {
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "propiedadId"
                        vm.onFormFieldChange("propiedadId", "p1")
                        vm.formState.value.errors.containsKey("propiedadId") shouldBe false
                    }
                }

                "onFechaInicioChange clears fechaInicio error" {
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "fechaInicio"
                        vm.onFechaInicioChange(LocalDate.of(2025, 1, 1))
                        vm.formState.value.errors.containsKey("fechaInicio") shouldBe false
                    }
                }
            }

        "renew flow" -
            {
                "showRenew opens dialog and dismissRenew closes it" {
                    val contratosRepo =
                        FakeContratosRepository(initialData = listOf(sampleContrato("c1")))
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.loadDetail("c1")
                        advanceUntilIdle()

                        vm.showRenew()
                        vm.showRenewDialog.value shouldBe true

                        vm.dismissRenew()
                        vm.showRenewDialog.value shouldBe false
                    }
                }

                "confirmRenew with blank fields shows errors" {
                    val contratosRepo =
                        FakeContratosRepository(initialData = listOf(sampleContrato("c1")))
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.loadDetail("c1")
                        advanceUntilIdle()

                        vm.showRenew()
                        vm.confirmRenew()
                        advanceUntilIdle()

                        vm.renewForm.value.errors shouldContainKey "fechaFin"
                        vm.renewForm.value.errors shouldContainKey "montoMensual"
                        vm.showRenewDialog.value shouldBe true
                        contratosRepo.renewCallCount shouldBe 0
                    }
                }

                "confirmRenew with valid data calls repository and closes dialog" {
                    val contratosRepo =
                        FakeContratosRepository(initialData = listOf(sampleContrato("c1")))
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.loadDetail("c1")
                        advanceUntilIdle()

                        vm.showRenew()
                        vm.onRenewFechaFinChange(LocalDate.of(2026, 12, 31))
                        vm.onRenewMontoChange("20000")
                        vm.confirmRenew()
                        advanceUntilIdle()

                        vm.showRenewDialog.value shouldBe false
                        vm.successMessage.value shouldBe "Contrato renovado"
                        contratosRepo.renewCallCount shouldBe 1
                    }
                }

                "onRenewFechaFinChange clears fechaFin error" {
                    val contratosRepo =
                        FakeContratosRepository(initialData = listOf(sampleContrato("c1")))
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.loadDetail("c1")
                        advanceUntilIdle()

                        vm.showRenew()
                        vm.confirmRenew()
                        advanceUntilIdle()

                        vm.renewForm.value.errors shouldContainKey "fechaFin"
                        vm.onRenewFechaFinChange(LocalDate.of(2026, 12, 31))
                        vm.renewForm.value.errors.containsKey("fechaFin") shouldBe false
                    }
                }
            }

        "terminate flow" -
            {
                "showTerminate opens dialog and dismissTerminate closes it" {
                    val contratosRepo =
                        FakeContratosRepository(initialData = listOf(sampleContrato("c1")))
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.loadDetail("c1")
                        advanceUntilIdle()

                        vm.showTerminate()
                        vm.showTerminateDialog.value shouldBe true

                        vm.dismissTerminate()
                        vm.showTerminateDialog.value shouldBe false
                    }
                }

                "confirmTerminate calls repository and closes dialog" {
                    val contratosRepo =
                        FakeContratosRepository(initialData = listOf(sampleContrato("c1")))
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.loadDetail("c1")
                        advanceUntilIdle()

                        vm.showTerminate()
                        vm.confirmTerminate()
                        advanceUntilIdle()

                        vm.showTerminateDialog.value shouldBe false
                        vm.successMessage.value shouldBe "Contrato terminado"
                        contratosRepo.terminateCallCount shouldBe 1
                    }
                }
            }

        "expiring filter (Req 5.6)" -
            {
                "expiring state contains only contracts within threshold" {
                    val today = LocalDate.now()
                    val expiringSoon = sampleContrato("c1", fechaFin = today.plusDays(15))
                    val expiringLater = sampleContrato("c2", fechaFin = today.plusDays(60))
                    val contratosRepo =
                        FakeContratosRepository(
                            initialData = listOf(expiringSoon, expiringLater),
                            expiringData = listOf(expiringSoon),
                        )

                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        val expiringList = vm.expiring.value
                        expiringList shouldHaveSize 1
                        expiringList.first().contrato.id shouldBe "c1"
                    }
                }

                "expiring state is empty when no contracts are expiring" {
                    val contratosRepo =
                        FakeContratosRepository(
                            initialData =
                                listOf(
                                    sampleContrato("c1", fechaFin = LocalDate.now().plusDays(90))
                                ),
                            expiringData = emptyList(),
                        )

                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.expiring.value shouldHaveSize 0
                    }
                }
            }

        "CRUD state transitions" -
            {
                "create with valid data succeeds" {
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFormFieldChange("propiedadId", "p1")
                        vm.onFormFieldChange("inquilinoId", "i1")
                        vm.onFechaInicioChange(LocalDate.of(2025, 1, 1))
                        vm.onFechaFinChange(LocalDate.of(2025, 12, 31))
                        vm.onFormFieldChange("montoMensual", "15000")

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe true
                        vm.formState.value.errors.shouldBeEmpty()
                        vm.formState.value.isSubmitting shouldBe false
                        vm.successMessage.value shouldBe "Creado correctamente"
                        contratosRepo.createCallCount shouldBe 1
                    }
                }

                "create failure sets general error on form" {
                    val contratosRepo =
                        FakeContratosRepository(createError = RuntimeException("DB error"))
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFormFieldChange("propiedadId", "p1")
                        vm.onFormFieldChange("inquilinoId", "i1")
                        vm.onFechaInicioChange(LocalDate.of(2025, 1, 1))
                        vm.onFechaFinChange(LocalDate.of(2025, 12, 31))
                        vm.onFormFieldChange("montoMensual", "15000")

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe false
                        vm.formState.value.errors shouldContainKey "general"
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }

                "delete flow sets target, confirms, and clears" {
                    val target = sampleContrato("c1")
                    val contratosRepo = FakeContratosRepository(initialData = listOf(target))
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.requestDelete(target)
                        vm.deleteTarget.value.shouldNotBeNull()
                        vm.deleteTarget.value!!.id shouldBe "c1"

                        vm.confirmDelete()
                        advanceUntilIdle()

                        vm.deleteTarget.value.shouldBeNull()
                        vm.successMessage.value shouldBe "Eliminado correctamente"
                        contratosRepo.deleteCallCount shouldBe 1
                    }
                }

                "dismissDelete clears delete target without deleting" {
                    val target = sampleContrato("c1")
                    val contratosRepo = FakeContratosRepository(initialData = listOf(target))
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.requestDelete(target)
                        vm.deleteTarget.value.shouldNotBeNull()

                        vm.dismissDelete()
                        vm.deleteTarget.value.shouldBeNull()
                        contratosRepo.deleteCallCount shouldBe 0
                    }
                }
            }

        "detail state" -
            {
                "loadDetail emits Success for existing contrato" {
                    val propiedades = listOf(samplePropiedad("p1", titulo = "Mi Casa"))
                    val inquilinos =
                        listOf(sampleInquilino("i1", nombre = "Ana", apellido = "Lopez"))
                    val contratos =
                        listOf(sampleContrato("c1", propiedadId = "p1", inquilinoId = "i1"))
                    val contratosRepo = FakeContratosRepository(initialData = contratos)
                    val propRepo = FakePropiedadesRepository(initialData = propiedades)
                    val inqRepo = FakeInquilinosRepository(initialData = inquilinos)

                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo, propRepo, inqRepo)
                        advanceUntilIdle()

                        vm.loadDetail("c1")
                        advanceUntilIdle()

                        val detail = vm.detailState.value
                        detail.shouldBeInstanceOf<ContratoDetailUiState.Success>()
                        detail.contrato.contrato.id shouldBe "c1"
                        detail.contrato.propiedadTitulo shouldBe "Mi Casa"
                        detail.contrato.inquilinoNombre shouldBe "Ana Lopez"
                    }
                }

                "loadDetail emits NotFound for missing contrato" {
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.loadDetail("nonexistent")
                        advanceUntilIdle()

                        vm.detailState.value.shouldBeInstanceOf<ContratoDetailUiState.NotFound>()
                    }
                }
            }

        "form initialization" -
            {
                "initCreateForm resets form to defaults" {
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.onFormFieldChange("propiedadId", "p1")
                        vm.initCreateForm()

                        vm.formState.value.propiedadId shouldBe ""
                        vm.formState.value.errors.shouldBeEmpty()
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }

                "initEditForm populates form from contrato" {
                    val contrato =
                        sampleContrato(
                            "c1",
                            propiedadId = "p1",
                            inquilinoId = "i1",
                            montoMensual = BigDecimal("25000.00"),
                        )
                    val contratosRepo = FakeContratosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(contratosRepo)
                        advanceUntilIdle()

                        vm.initEditForm(contrato)

                        vm.formState.value.propiedadId shouldBe "p1"
                        vm.formState.value.inquilinoId shouldBe "i1"
                        vm.formState.value.montoMensual shouldBe "25000.00"
                        vm.formState.value.fechaInicio shouldBe contrato.fechaInicio
                        vm.formState.value.fechaFin shouldBe contrato.fechaFin
                    }
                }
            }

        "clearSuccessMessage resets success message" {
            val contratosRepo = FakeContratosRepository(initialData = listOf(sampleContrato("c1")))
            runTest(testDispatcher) {
                val vm = createViewModel(contratosRepo)
                advanceUntilIdle()

                vm.requestDelete(sampleContrato("c1"))
                vm.confirmDelete()
                advanceUntilIdle()

                vm.successMessage.value.shouldNotBeNull()
                vm.clearSuccessMessage()
                vm.successMessage.value.shouldBeNull()
            }
        }
    })

private fun createViewModel(
    contratosRepo: FakeContratosRepository,
    propRepo: FakePropiedadesRepository = FakePropiedadesRepository(),
    inqRepo: FakeInquilinosRepository = FakeInquilinosRepository(),
    connectivity: FakeConnectivityObserver = FakeConnectivityObserver(),
): ContratosViewModel = ContratosViewModel(contratosRepo, propRepo, inqRepo, connectivity)

private fun sampleContrato(
    id: String,
    propiedadId: String = "p1",
    inquilinoId: String = "i1",
    fechaInicio: LocalDate = LocalDate.of(2025, 1, 1),
    fechaFin: LocalDate = LocalDate.of(2025, 12, 31),
    montoMensual: BigDecimal = BigDecimal("15000.00"),
    estado: String = "activo",
) =
    Contrato(
        id = id,
        propiedadId = propiedadId,
        inquilinoId = inquilinoId,
        fechaInicio = fechaInicio,
        fechaFin = fechaFin,
        montoMensual = montoMensual,
        deposito = null,
        moneda = "DOP",
        estado = estado,
        createdAt = Instant.now(),
        updatedAt = Instant.now(),
        isPendingSync = false,
    )

private fun samplePropiedad(id: String, titulo: String = "Propiedad $id") =
    Propiedad(
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

private fun sampleInquilino(
    id: String,
    nombre: String = "Inquilino $id",
    apellido: String = "Apellido $id",
) =
    Inquilino(
        id = id,
        nombre = nombre,
        apellido = apellido,
        email = null,
        telefono = null,
        cedula = "001-000000$id-0",
        contactoEmergencia = null,
        notas = null,
        createdAt = Instant.now(),
        updatedAt = Instant.now(),
        isPendingSync = false,
    )

private class FakeConnectivityObserver(online: Boolean = true) : ConnectivityObserver {
    override val isOnline: StateFlow<Boolean> = MutableStateFlow(online).asStateFlow()
}

private val stubJson = Json { ignoreUnknownKeys = true }

@Suppress("UNCHECKED_CAST")
private fun stubContratoDao(): com.propmanager.core.database.dao.ContratoDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.ContratoDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.ContratoDao::class.java),
    ) { _, _, _ ->
        error("stub")
    } as com.propmanager.core.database.dao.ContratoDao

@Suppress("UNCHECKED_CAST")
private fun stubSyncQueueDao(): com.propmanager.core.database.dao.SyncQueueDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.SyncQueueDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.SyncQueueDao::class.java),
    ) { _, _, _ ->
        error("stub")
    } as com.propmanager.core.database.dao.SyncQueueDao

@Suppress("UNCHECKED_CAST")
private fun stubContratosApiService(): com.propmanager.core.network.api.ContratosApiService =
    Proxy.newProxyInstance(
        com.propmanager.core.network.api.ContratosApiService::class.java.classLoader,
        arrayOf(com.propmanager.core.network.api.ContratosApiService::class.java),
    ) { _, _, _ ->
        error("stub")
    } as com.propmanager.core.network.api.ContratosApiService

@Suppress("UNCHECKED_CAST")
private fun stubPropiedadDao(): com.propmanager.core.database.dao.PropiedadDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.PropiedadDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.PropiedadDao::class.java),
    ) { _, _, _ ->
        error("stub")
    } as com.propmanager.core.database.dao.PropiedadDao

@Suppress("UNCHECKED_CAST")
private fun stubPropiedadesApiService(): com.propmanager.core.network.api.PropiedadesApiService =
    Proxy.newProxyInstance(
        com.propmanager.core.network.api.PropiedadesApiService::class.java.classLoader,
        arrayOf(com.propmanager.core.network.api.PropiedadesApiService::class.java),
    ) { _, _, _ ->
        error("stub")
    } as com.propmanager.core.network.api.PropiedadesApiService

@Suppress("UNCHECKED_CAST")
private fun stubInquilinoDao(): com.propmanager.core.database.dao.InquilinoDao =
    Proxy.newProxyInstance(
        com.propmanager.core.database.dao.InquilinoDao::class.java.classLoader,
        arrayOf(com.propmanager.core.database.dao.InquilinoDao::class.java),
    ) { _, _, _ ->
        error("stub")
    } as com.propmanager.core.database.dao.InquilinoDao

@Suppress("UNCHECKED_CAST")
private fun stubInquilinosApiService(): com.propmanager.core.network.api.InquilinosApiService =
    Proxy.newProxyInstance(
        com.propmanager.core.network.api.InquilinosApiService::class.java.classLoader,
        arrayOf(com.propmanager.core.network.api.InquilinosApiService::class.java),
    ) { _, _, _ ->
        error("stub")
    } as com.propmanager.core.network.api.InquilinosApiService

private class FakeContratosRepository(
    private val initialData: List<Contrato> = emptyList(),
    private val expiringData: List<Contrato>? = null,
    private val createError: Throwable? = null,
) :
    ContratosRepository(
        dao = stubContratoDao(),
        syncQueueDao = stubSyncQueueDao(),
        apiService = stubContratosApiService(),
        json = stubJson,
    ) {
    var createCallCount = 0
        private set

    var renewCallCount = 0
        private set

    var terminateCallCount = 0
        private set

    var deleteCallCount = 0
        private set

    private val store = MutableStateFlow(initialData)

    override fun observeAll(): Flow<List<Contrato>> = store

    override fun observeById(id: String): Flow<Contrato?> =
        store.map { list -> list.find { it.id == id } }

    override fun observeExpiring(daysThreshold: Int): Flow<List<Contrato>> =
        MutableStateFlow(expiringData ?: initialData)

    override suspend fun create(request: CreateContratoRequest): Result<Contrato> {
        createCallCount++
        if (createError != null) return Result.failure(createError)
        val c =
            Contrato(
                id = "new-$createCallCount",
                propiedadId = request.propiedadId,
                inquilinoId = request.inquilinoId,
                fechaInicio = LocalDate.parse(request.fechaInicio),
                fechaFin = LocalDate.parse(request.fechaFin),
                montoMensual = request.montoMensual.toBigDecimal(),
                deposito = request.deposito?.toBigDecimalOrNull(),
                moneda = request.moneda ?: "DOP",
                estado = "activo",
                createdAt = Instant.now(),
                updatedAt = Instant.now(),
                isPendingSync = true,
            )
        store.value = store.value + c
        return Result.success(c)
    }

    override suspend fun renew(id: String, request: RenovarContratoRequest): Result<Unit> {
        renewCallCount++
        return Result.success(Unit)
    }

    override suspend fun terminate(id: String, request: TerminarContratoRequest): Result<Unit> {
        terminateCallCount++
        return Result.success(Unit)
    }

    override suspend fun delete(id: String): Result<Unit> {
        deleteCallCount++
        store.value = store.value.filter { it.id != id }
        return Result.success(Unit)
    }
}

private class FakePropiedadesRepository(private val initialData: List<Propiedad> = emptyList()) :
    PropiedadesRepository(
        dao = stubPropiedadDao(),
        syncQueueDao = stubSyncQueueDao(),
        apiService = stubPropiedadesApiService(),
        json = stubJson,
    ) {
    override fun observeAll(): Flow<List<Propiedad>> = MutableStateFlow(initialData)
}

private class FakeInquilinosRepository(private val initialData: List<Inquilino> = emptyList()) :
    InquilinosRepository(
        dao = stubInquilinoDao(),
        syncQueueDao = stubSyncQueueDao(),
        apiService = stubInquilinosApiService(),
        json = stubJson,
    ) {
    override fun observeAll(): Flow<List<Inquilino>> = MutableStateFlow(initialData)
}
