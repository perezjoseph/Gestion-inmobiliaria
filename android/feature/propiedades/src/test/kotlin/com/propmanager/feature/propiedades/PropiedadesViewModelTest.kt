package com.propmanager.feature.propiedades

import com.propmanager.core.data.repository.PropiedadesRepository
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.dto.CreatePropiedadRequest
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
import java.lang.reflect.Proxy
import java.math.BigDecimal
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
 * Unit tests for PropiedadesViewModel.
 *
 * Validates: Requirements 3.1, 3.2, 3.7
 */
@OptIn(ExperimentalCoroutinesApi::class)
class PropiedadesViewModelTest :
    FreeSpec({
        val testDispatcher = StandardTestDispatcher()

        beforeEach { Dispatchers.setMain(testDispatcher) }

        afterEach { Dispatchers.resetMain() }

        "list state" -
            {
                "emits Success with propiedades from repository" {
                    val repo =
                        FakePropiedadesRepository(
                            initialData =
                                listOf(
                                    samplePropiedad("1", ciudad = "Santiago"),
                                    samplePropiedad("2", ciudad = "Santo Domingo"),
                                )
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        val state = vm.propiedades.value
                        state.shouldBeInstanceOf<PropiedadesUiState.Success>()
                        state.propiedades shouldHaveSize 2
                    }
                }

                "emits Success with empty list when no propiedades" {
                    val repo = FakePropiedadesRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        val state = vm.propiedades.value
                        state.shouldBeInstanceOf<PropiedadesUiState.Success>()
                        state.propiedades shouldHaveSize 0
                    }
                }
            }

        "filter application" -
            {
                "updateFilter changes filter state and filters list by ciudad" {
                    val repo =
                        FakePropiedadesRepository(
                            initialData =
                                listOf(
                                    samplePropiedad("1", ciudad = "Santiago"),
                                    samplePropiedad("2", ciudad = "Santo Domingo"),
                                    samplePropiedad("3", ciudad = "Santiago"),
                                )
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.updateFilter(ciudad = "Santiago")
                        advanceUntilIdle()

                        vm.filters.value.ciudad shouldBe "Santiago"
                        val state = vm.propiedades.value
                        state.shouldBeInstanceOf<PropiedadesUiState.Success>()
                        state.propiedades shouldHaveSize 2
                        state.propiedades.all { it.ciudad == "Santiago" } shouldBe true
                    }
                }

                "updateFilter filters by estado" {
                    val repo =
                        FakePropiedadesRepository(
                            initialData =
                                listOf(
                                    samplePropiedad("1", estado = "disponible"),
                                    samplePropiedad("2", estado = "ocupada"),
                                )
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.updateFilter(estado = "disponible")
                        advanceUntilIdle()

                        val state = vm.propiedades.value
                        state.shouldBeInstanceOf<PropiedadesUiState.Success>()
                        state.propiedades shouldHaveSize 1
                        state.propiedades.first().estado shouldBe "disponible"
                    }
                }

                "updateFilter filters by tipoPropiedad" {
                    val repo =
                        FakePropiedadesRepository(
                            initialData =
                                listOf(
                                    samplePropiedad("1", tipoPropiedad = "apartamento"),
                                    samplePropiedad("2", tipoPropiedad = "casa"),
                                    samplePropiedad("3", tipoPropiedad = "apartamento"),
                                )
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.updateFilter(tipoPropiedad = "apartamento")
                        advanceUntilIdle()

                        val state = vm.propiedades.value
                        state.shouldBeInstanceOf<PropiedadesUiState.Success>()
                        state.propiedades shouldHaveSize 2
                    }
                }

                "clearFilters resets all filters and shows all propiedades" {
                    val repo =
                        FakePropiedadesRepository(
                            initialData =
                                listOf(
                                    samplePropiedad("1", ciudad = "Santiago"),
                                    samplePropiedad("2", ciudad = "Santo Domingo"),
                                )
                        )
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.updateFilter(ciudad = "Santiago")
                        advanceUntilIdle()

                        vm.clearFilters()
                        advanceUntilIdle()

                        vm.filters.value shouldBe PropiedadesFilterState()
                        val state = vm.propiedades.value
                        state.shouldBeInstanceOf<PropiedadesUiState.Success>()
                        state.propiedades shouldHaveSize 2
                    }
                }
            }

        "CRUD state transitions" -
            {
                "create with valid data succeeds and calls onSuccess" {
                    val repo = FakePropiedadesRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("titulo", "Casa Bonita")
                        vm.onFieldChange("direccion", "Calle 1 #23")
                        vm.onFieldChange("ciudad", "Santiago")
                        vm.onFieldChange("provincia", "Santiago")
                        vm.onFieldChange("tipoPropiedad", "casa")
                        vm.onFieldChange("precio", "25000.00")

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
                    val existing = samplePropiedad("1", titulo = "Original")
                    val repo = FakePropiedadesRepository(initialData = listOf(existing))
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initEditForm(existing)
                        vm.onFieldChange("titulo", "Actualizada")

                        var successCalled = false
                        vm.save { successCalled = true }
                        advanceUntilIdle()

                        successCalled shouldBe true
                        vm.successMessage.value shouldBe "Actualizado correctamente"
                        repo.updateCallCount shouldBe 1
                    }
                }

                "delete flow sets target, confirms, and clears" {
                    val target = samplePropiedad("1")
                    val repo = FakePropiedadesRepository(initialData = listOf(target))
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
                    val target = samplePropiedad("1")
                    val repo = FakePropiedadesRepository(initialData = listOf(target))
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
                    val repo = FakePropiedadesRepository(createError = RuntimeException("DB error"))
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("titulo", "Test")
                        vm.onFieldChange("direccion", "Calle 1")
                        vm.onFieldChange("ciudad", "Santiago")
                        vm.onFieldChange("provincia", "Santiago")
                        vm.onFieldChange("tipoPropiedad", "casa")
                        vm.onFieldChange("precio", "1000")

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
                    val repo = FakePropiedadesRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        val errors = vm.formState.value.errors
                        errors shouldContainKey "titulo"
                        errors shouldContainKey "direccion"
                        errors shouldContainKey "ciudad"
                        errors shouldContainKey "provincia"
                        errors shouldContainKey "tipoPropiedad"
                        errors shouldContainKey "precio"
                        errors["titulo"]!!.shouldNotBeBlank()
                        repo.createCallCount shouldBe 0
                    }
                }

                "save with invalid precio shows precio error" {
                    val repo = FakePropiedadesRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("titulo", "Casa")
                        vm.onFieldChange("direccion", "Calle 1")
                        vm.onFieldChange("ciudad", "Santiago")
                        vm.onFieldChange("provincia", "Santiago")
                        vm.onFieldChange("tipoPropiedad", "casa")
                        vm.onFieldChange("precio", "-100")

                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "precio"
                        repo.createCallCount shouldBe 0
                    }
                }

                "save with zero precio shows precio error" {
                    val repo = FakePropiedadesRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.onFieldChange("titulo", "Casa")
                        vm.onFieldChange("direccion", "Calle 1")
                        vm.onFieldChange("ciudad", "Santiago")
                        vm.onFieldChange("provincia", "Santiago")
                        vm.onFieldChange("tipoPropiedad", "casa")
                        vm.onFieldChange("precio", "0")

                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "precio"
                        repo.createCallCount shouldBe 0
                    }
                }

                "onFieldChange clears error for that field" {
                    val repo = FakePropiedadesRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initCreateForm()
                        vm.save {}
                        advanceUntilIdle()

                        vm.formState.value.errors shouldContainKey "titulo"

                        vm.onFieldChange("titulo", "Casa Nueva")
                        vm.formState.value.errors.containsKey("titulo") shouldBe false
                    }
                }
            }

        "detail state" -
            {
                "loadDetail emits Success for existing propiedad" {
                    val prop = samplePropiedad("42", titulo = "Mi Casa")
                    val repo = FakePropiedadesRepository(initialData = listOf(prop))
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.loadDetail("42")
                        advanceUntilIdle()

                        val detail = vm.detailState.value
                        detail.shouldBeInstanceOf<PropiedadDetailUiState.Success>()
                        detail.propiedad.id shouldBe "42"
                        detail.propiedad.titulo shouldBe "Mi Casa"
                    }
                }

                "loadDetail emits NotFound for missing propiedad" {
                    val repo = FakePropiedadesRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.loadDetail("nonexistent")
                        advanceUntilIdle()

                        vm.detailState.value.shouldBeInstanceOf<PropiedadDetailUiState.NotFound>()
                    }
                }
            }

        "form initialization" -
            {
                "initCreateForm resets form to defaults" {
                    val repo = FakePropiedadesRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.onFieldChange("titulo", "Something")
                        vm.initCreateForm()

                        vm.formState.value.titulo shouldBe ""
                        vm.formState.value.errors.shouldBeEmpty()
                        vm.formState.value.isSubmitting shouldBe false
                    }
                }

                "initEditForm populates form from propiedad" {
                    val prop =
                        samplePropiedad(
                            "1",
                            titulo = "Casa Bonita",
                            ciudad = "Santiago",
                            provincia = "Santiago",
                            tipoPropiedad = "casa",
                            precio = BigDecimal("25000.00"),
                        )
                    val repo = FakePropiedadesRepository()
                    runTest(testDispatcher) {
                        val vm = createViewModel(repo)
                        advanceUntilIdle()

                        vm.initEditForm(prop)

                        vm.formState.value.titulo shouldBe "Casa Bonita"
                        vm.formState.value.ciudad shouldBe "Santiago"
                        vm.formState.value.provincia shouldBe "Santiago"
                        vm.formState.value.tipoPropiedad shouldBe "casa"
                        vm.formState.value.precio shouldBe "25000.00"
                    }
                }
            }

        "clearSuccessMessage resets success message" {
            val repo = FakePropiedadesRepository(initialData = listOf(samplePropiedad("1")))
            runTest(testDispatcher) {
                val vm = createViewModel(repo)
                advanceUntilIdle()

                vm.requestDelete(samplePropiedad("1"))
                vm.confirmDelete()
                advanceUntilIdle()

                vm.successMessage.value.shouldNotBeNull()
                vm.clearSuccessMessage()
                vm.successMessage.value.shouldBeNull()
            }
        }
    })

private fun createViewModel(
    repo: FakePropiedadesRepository,
    connectivity: FakeConnectivityObserver = FakeConnectivityObserver(),
): PropiedadesViewModel = PropiedadesViewModel(repo, connectivity)

private fun samplePropiedad(
    id: String,
    titulo: String = "Propiedad $id",
    ciudad: String = "Santiago",
    provincia: String = "Santiago",
    tipoPropiedad: String = "apartamento",
    estado: String = "disponible",
    precio: BigDecimal = BigDecimal("15000.00"),
) =
    Propiedad(
        id = id,
        titulo = titulo,
        descripcion = null,
        direccion = "Calle $id",
        ciudad = ciudad,
        provincia = provincia,
        tipoPropiedad = tipoPropiedad,
        habitaciones = 2,
        banos = 1,
        areaM2 = BigDecimal("80"),
        precio = precio,
        moneda = "DOP",
        estado = estado,
        imagenes = emptyList(),
        createdAt = Instant.now(),
        updatedAt = Instant.now(),
        isPendingSync = false,
    )

private class FakeConnectivityObserver(online: Boolean = true) : ConnectivityObserver {
    override val isOnline: StateFlow<Boolean> = MutableStateFlow(online).asStateFlow()
}

@Suppress("UNCHECKED_CAST")
private class FakePropiedadesRepository(
    private val initialData: List<Propiedad> = emptyList(),
    private val createError: Throwable? = null,
    private val updateError: Throwable? = null,
) :
    PropiedadesRepository(
        dao =
            Proxy.newProxyInstance(
                com.propmanager.core.database.dao.PropiedadDao::class.java.classLoader,
                arrayOf(com.propmanager.core.database.dao.PropiedadDao::class.java),
            ) { _, _, _ ->
                error("stub")
            } as com.propmanager.core.database.dao.PropiedadDao,
        syncQueueDao =
            Proxy.newProxyInstance(
                com.propmanager.core.database.dao.SyncQueueDao::class.java.classLoader,
                arrayOf(com.propmanager.core.database.dao.SyncQueueDao::class.java),
            ) { _, _, _ ->
                error("stub")
            } as com.propmanager.core.database.dao.SyncQueueDao,
        apiService =
            Proxy.newProxyInstance(
                com.propmanager.core.network.api.PropiedadesApiService::class.java.classLoader,
                arrayOf(com.propmanager.core.network.api.PropiedadesApiService::class.java),
            ) { _, _, _ ->
                error("stub")
            } as com.propmanager.core.network.api.PropiedadesApiService,
        json = Json { ignoreUnknownKeys = true },
    ) {
    var createCallCount = 0
        private set

    var updateCallCount = 0
        private set

    var deleteCallCount = 0
        private set

    private val store = MutableStateFlow(initialData)

    override fun observeAll(): Flow<List<Propiedad>> = store

    override fun observeFiltered(
        ciudad: String?,
        estado: String?,
        tipoPropiedad: String?,
    ): Flow<List<Propiedad>> =
        store.map { list ->
            list.filter { p ->
                (ciudad == null || p.ciudad == ciudad) &&
                    (estado == null || p.estado == estado) &&
                    (tipoPropiedad == null || p.tipoPropiedad == tipoPropiedad)
            }
        }

    override fun observeById(id: String): Flow<Propiedad?> =
        store.map { list -> list.find { it.id == id } }

    override suspend fun create(request: CreatePropiedadRequest): Result<Propiedad> {
        createCallCount++
        if (createError != null) return Result.failure(createError)
        val p =
            Propiedad(
                id = "new-$createCallCount",
                titulo = request.titulo,
                descripcion = request.descripcion,
                direccion = request.direccion,
                ciudad = request.ciudad,
                provincia = request.provincia,
                tipoPropiedad = request.tipoPropiedad,
                habitaciones = request.habitaciones,
                banos = request.banos,
                areaM2 = request.areaM2?.toBigDecimalOrNull(),
                precio = request.precio.toBigDecimal(),
                moneda = request.moneda ?: "DOP",
                estado = request.estado ?: "disponible",
                imagenes = emptyList(),
                createdAt = Instant.now(),
                updatedAt = Instant.now(),
                isPendingSync = true,
            )
        store.value = store.value + p
        return Result.success(p)
    }

    override suspend fun update(id: String, request: UpdatePropiedadRequest): Result<Unit> {
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
