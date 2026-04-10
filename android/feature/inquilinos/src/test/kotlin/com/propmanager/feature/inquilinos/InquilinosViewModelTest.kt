package com.propmanager.feature.inquilinos

import com.propmanager.core.data.repository.InquilinosRepository
import com.propmanager.core.model.Inquilino
import com.propmanager.core.model.dto.CreateInquilinoRequest
import com.propmanager.core.model.dto.UpdateInquilinoRequest
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
import java.time.Instant
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
 * Unit tests for InquilinosViewModel.
 *
 * Validates: Requirements 4.1, 4.2, 4.6
 */
@OptIn(ExperimentalCoroutinesApi::class)
class InquilinosViewModelTest :
    FreeSpec({
        val testDispatcher = StandardTestDispatcher()

        beforeEach { Dispatchers.setMain(testDispatcher) }

        afterEach { Dispatchers.resetMain() }

        "list state" -
            {
                "emits Success with inquilinos from repository" {
                    val repo =
                        FakeInquilinosRepository(
                            initialData =
                                listOf(
                                    sampleInquilino("1", nombre = "Juan"),
                                    sampleInquilino("2", nombre = "Maria"),
                                )
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        val state = vm.inquilinos.value
                        state.shouldBeInstanceOf<InquilinosUiState.Success>()
                        state.inquilinos shouldHaveSize 2
                    }
                }

                "emits Success with empty list when no inquilinos" {
                    val repo = FakeInquilinosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        val state = vm.inquilinos.value
                        state.shouldBeInstanceOf<InquilinosUiState.Success>()
                        state.inquilinos shouldHaveSize 0
                    }
                }
            }

        "search filtering" -
            {
                "search by nombre filters list" {
                    val repo =
                        FakeInquilinosRepository(
                            initialData =
                                listOf(
                                    sampleInquilino(
                                        "1",
                                        nombre = "Juan",
                                        apellido = "Perez",
                                        cedula = "001-0000001-1",
                                    ),
                                    sampleInquilino(
                                        "2",
                                        nombre = "Maria",
                                        apellido = "Lopez",
                                        cedula = "001-0000002-2",
                                    ),
                                    sampleInquilino(
                                        "3",
                                        nombre = "Juanita",
                                        apellido = "Garcia",
                                        cedula = "001-0000003-3",
                                    ),
                                )
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.onSearchChange("Juan")
                        advanceUntilIdle()

                        vm.searchQuery.value shouldBe "Juan"
                        val state = vm.inquilinos.value
                        state.shouldBeInstanceOf<InquilinosUiState.Success>()
                        state.inquilinos shouldHaveSize 2
                        state.inquilinos.all { it.nombre.contains("Juan") } shouldBe true
                    }
                }

                "search by apellido filters list" {
                    val repo =
                        FakeInquilinosRepository(
                            initialData =
                                listOf(
                                    sampleInquilino("1", nombre = "Juan", apellido = "Perez"),
                                    sampleInquilino("2", nombre = "Maria", apellido = "Lopez"),
                                )
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.onSearchChange("Lopez")
                        advanceUntilIdle()

                        val state = vm.inquilinos.value
                        state.shouldBeInstanceOf<InquilinosUiState.Success>()
                        state.inquilinos shouldHaveSize 1
                        state.inquilinos.first().apellido shouldBe "Lopez"
                    }
                }

                "search by cedula filters list" {
                    val repo =
                        FakeInquilinosRepository(
                            initialData =
                                listOf(
                                    sampleInquilino("1", cedula = "001-0000001-1"),
                                    sampleInquilino("2", cedula = "002-0000002-2"),
                                )
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.onSearchChange("002")
                        advanceUntilIdle()

                        val state = vm.inquilinos.value
                        state.shouldBeInstanceOf<InquilinosUiState.Success>()
                        state.inquilinos shouldHaveSize 1
                        state.inquilinos.first().cedula shouldBe "002-0000002-2"
                    }
                }

                "empty search shows all inquilinos" {
                    val repo =
                        FakeInquilinosRepository(
                            initialData = listOf(sampleInquilino("1"), sampleInquilino("2"))
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.onSearchChange("test")
                        advanceUntilIdle()

                        vm.onSearchChange("")
                        advanceUntilIdle()

                        val state = vm.inquilinos.value
                        state.shouldBeInstanceOf<InquilinosUiState.Success>()
                        state.inquilinos shouldHaveSize 2
                    }
                }
            }

        "CRUD state transitions" -
            {
                "create with valid data succeeds and calls onSuccess" {
                    val repo = FakeInquilinosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("nombre", "Juan")
                        vm.onFieldChange("apellido", "Perez")
                        vm.onFieldChange("cedula", "001-0000001-1")

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
                    val existing = sampleInquilino("1", nombre = "Juan")
                    val repo = FakeInquilinosRepository(initialData = listOf(existing))
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initEditForm(existing)
                        vm.onFieldChange("nombre", "Pedro")

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe true
                        vm.successMessage.value shouldBe "Actualizado correctamente"
                        repo.updateCallCount shouldBe 1
                    }
                }

                "delete flow sets target, confirms, and clears" {
                    val target = sampleInquilino("1")
                    val repo = FakeInquilinosRepository(initialData = listOf(target))
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
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
                    val target = sampleInquilino("1")
                    val repo = FakeInquilinosRepository(initialData = listOf(target))
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.requestDelete(target)
                        vm.deleteTarget.value.shouldNotBeNull()

                        vm.dismissDelete()
                        vm.deleteTarget.value.shouldBeNull()
                        repo.deleteCallCount shouldBe 0
                    }
                }

                "create failure sets general error on form" {
                    val repo = FakeInquilinosRepository(createError = RuntimeException("DB error"))
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("nombre", "Juan")
                        vm.onFieldChange("apellido", "Perez")
                        vm.onFieldChange("cedula", "001-0000001-1")

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe false
                        vm.formState.value.errors shouldContainKey "general"
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }
            }

        "validation errors" -
            {
                "save with blank required fields shows validation errors and does not call repository" {
                    val repo = FakeInquilinosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        val errors = vm.formState.value.errors
                        errors shouldContainKey "nombre"
                        errors shouldContainKey "apellido"
                        errors shouldContainKey "cedula"
                        errors["nombre"]!!.shouldNotBeBlank()
                        repo.createCallCount shouldBe 0
                    }
                }

                "onFieldChange clears error for that field" {
                    val repo = FakeInquilinosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "nombre"

                        vm.onFieldChange("nombre", "Juan")
                        vm.formState.value.errors.containsKey("nombre") shouldBe false
                    }
                }
            }

        "OCR data pre-fill" -
            {
                "prefillFromOcr sets nombre, apellido, and cedula on form" {
                    val repo = FakeInquilinosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.prefillFromOcr(
                            nombre = "Juan Carlos",
                            apellido = "Perez Martinez",
                            cedula = "001-0000001-1",
                        )

                        vm.formState.value.nombre shouldBe "Juan Carlos"
                        vm.formState.value.apellido shouldBe "Perez Martinez"
                        vm.formState.value.cedula shouldBe "001-0000001-1"
                    }
                }

                "prefillFromOcr with null values preserves existing form data" {
                    val repo = FakeInquilinosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("nombre", "Existing")
                        vm.prefillFromOcr(nombre = null, apellido = "FromOcr", cedula = null)

                        vm.formState.value.nombre shouldBe "Existing"
                        vm.formState.value.apellido shouldBe "FromOcr"
                        vm.formState.value.cedula shouldBe ""
                    }
                }

                "prefillFromOcr data can be saved successfully" {
                    val repo = FakeInquilinosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.prefillFromOcr(
                            nombre = "Juan",
                            apellido = "Perez",
                            cedula = "001-0000001-1",
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
                    val repo = FakeInquilinosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.onFieldChange("nombre", "Something")
                        vm.initCreateForm()

                        vm.formState.value.nombre shouldBe ""
                        vm.formState.value.errors.shouldBeEmpty()
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }

                "initEditForm populates form from inquilino" {
                    val inquilino =
                        sampleInquilino(
                            "1",
                            nombre = "Juan",
                            apellido = "Perez",
                            cedula = "001-0000001-1",
                        )
                    val repo = FakeInquilinosRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initEditForm(inquilino)

                        vm.formState.value.nombre shouldBe "Juan"
                        vm.formState.value.apellido shouldBe "Perez"
                        vm.formState.value.cedula shouldBe "001-0000001-1"
                    }
                }
            }

        "clearSuccessMessage resets success message" {
            val repo = FakeInquilinosRepository(initialData = listOf(sampleInquilino("1")))
            runTest(testDispatcher) {
                val vm = createViewModel(repo)
                advanceUntilIdle()

                vm.requestDelete(sampleInquilino("1"))
                vm.confirmDelete()
                advanceUntilIdle()

                vm.successMessage.value.shouldNotBeNull()
                vm.clearSuccessMessage()
                vm.successMessage.value.shouldBeNull()
            }
        }
    })

private fun createViewModel(
    repo: FakeInquilinosRepository,
    connectivity: FakeConnectivityObserver = FakeConnectivityObserver(),
): InquilinosViewModel = InquilinosViewModel(repo, connectivity)

private fun sampleInquilino(
    id: String,
    nombre: String = "Inquilino $id",
    apellido: String = "Apellido $id",
    cedula: String = "001-000000$id-0",
    email: String? = null,
    telefono: String? = null,
) =
    Inquilino(
        id = id,
        nombre = nombre,
        apellido = apellido,
        email = email,
        telefono = telefono,
        cedula = cedula,
        contactoEmergencia = null,
        notas = null,
        createdAt = Instant.now(),
        updatedAt = Instant.now(),
        isPendingSync = false,
    )

private class FakeConnectivityObserver(online: Boolean = true) : ConnectivityObserver {
    override val isOnline: StateFlow<Boolean> = MutableStateFlow(online).asStateFlow()
}

@Suppress("UNCHECKED_CAST")
private class FakeInquilinosRepository(
    private val initialData: List<Inquilino> = emptyList(),
    private val createError: Throwable? = null,
    private val updateError: Throwable? = null,
) :
    InquilinosRepository(
        dao =
            Proxy.newProxyInstance(
                com.propmanager.core.database.dao.InquilinoDao::class.java.classLoader,
                arrayOf(com.propmanager.core.database.dao.InquilinoDao::class.java),
            ) { _, _, _ ->
                error("stub")
            } as com.propmanager.core.database.dao.InquilinoDao,
        syncQueueDao =
            Proxy.newProxyInstance(
                com.propmanager.core.database.dao.SyncQueueDao::class.java.classLoader,
                arrayOf(com.propmanager.core.database.dao.SyncQueueDao::class.java),
            ) { _, _, _ ->
                error("stub")
            } as com.propmanager.core.database.dao.SyncQueueDao,
        apiService =
            Proxy.newProxyInstance(
                com.propmanager.core.network.api.InquilinosApiService::class.java.classLoader,
                arrayOf(com.propmanager.core.network.api.InquilinosApiService::class.java),
            ) { _, _, _ ->
                error("stub")
            } as com.propmanager.core.network.api.InquilinosApiService,
        json = Json { ignoreUnknownKeys = true },
    ) {
    var createCallCount = 0
        private set

    var updateCallCount = 0
        private set

    var deleteCallCount = 0
        private set

    private val store = MutableStateFlow(initialData)

    override fun observeAll(): Flow<List<Inquilino>> = store

    override fun search(query: String): Flow<List<Inquilino>> =
        store.map { list ->
            list.filter { i ->
                i.nombre.contains(query, ignoreCase = true) ||
                    i.apellido.contains(query, ignoreCase = true) ||
                    i.cedula.contains(query, ignoreCase = true)
            }
        }

    override fun observeById(id: String): Flow<Inquilino?> =
        store.map { list -> list.find { it.id == id } }

    override suspend fun create(request: CreateInquilinoRequest): Result<Inquilino> {
        createCallCount++
        if (createError != null) return Result.failure(createError)
        val i =
            Inquilino(
                id = "new-$createCallCount",
                nombre = request.nombre,
                apellido = request.apellido,
                email = request.email,
                telefono = request.telefono,
                cedula = request.cedula,
                contactoEmergencia = request.contactoEmergencia,
                notas = request.notas,
                createdAt = Instant.now(),
                updatedAt = Instant.now(),
                isPendingSync = true,
            )
        store.value = store.value + i
        return Result.success(i)
    }

    override suspend fun update(id: String, request: UpdateInquilinoRequest): Result<Unit> {
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
