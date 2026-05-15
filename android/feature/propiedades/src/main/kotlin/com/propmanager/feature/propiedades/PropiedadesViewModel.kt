package com.propmanager.feature.propiedades

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.common.PropiedadValidator
import com.propmanager.core.data.repository.PropiedadesRepository
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.Unidad
import com.propmanager.core.model.ValidationResult
import com.propmanager.core.model.dto.CreatePropiedadRequest
import com.propmanager.core.model.dto.CreateUnidadRequest
import com.propmanager.core.model.dto.UpdatePropiedadRequest
import com.propmanager.core.model.dto.UpdateUnidadRequest
import com.propmanager.core.network.ConnectivityObserver
import com.propmanager.core.network.api.PropiedadesApiService
import dagger.hilt.android.lifecycle.HiltViewModel
import java.math.BigDecimal
import java.time.Instant
import javax.inject.Inject
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.toImmutableList
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class PropiedadesFilterState(
    val ciudad: String? = null,
    val estado: String? = null,
    val tipoPropiedad: String? = null,
)

data class PropiedadFormState(
    val titulo: String = "",
    val descripcion: String = "",
    val direccion: String = "",
    val ciudad: String = "",
    val provincia: String = "",
    val tipoPropiedad: String = "",
    val habitaciones: String = "",
    val banos: String = "",
    val areaM2: String = "",
    val precio: String = "",
    val moneda: String = "DOP",
    val estado: String = "disponible",
    val errors: Map<String, String> = emptyMap(),
    val isSubmitting: Boolean = false,
)

sealed interface PropiedadesUiState {
    data object Loading : PropiedadesUiState

    data class Success(val propiedades: List<Propiedad>) : PropiedadesUiState

    data class Error(val message: String) : PropiedadesUiState
}

sealed interface PropiedadDetailUiState {
    data object Loading : PropiedadDetailUiState

    data class Success(val propiedad: Propiedad) : PropiedadDetailUiState

    data class NotFound(val message: String) : PropiedadDetailUiState
}

sealed interface UnidadesUiState {
    data object Loading : UnidadesUiState

    data class Success(val unidades: ImmutableList<Unidad>) : UnidadesUiState

    data class Error(val message: String) : UnidadesUiState
}

data class UnidadFormState(
    val numeroUnidad: String = "",
    val piso: String = "",
    val habitaciones: String = "",
    val banos: String = "",
    val areaM2: String = "",
    val precio: String = "",
    val moneda: String = "DOP",
    val estado: String = "disponible",
    val descripcion: String = "",
    val errors: Map<String, String> = emptyMap(),
    val isSubmitting: Boolean = false,
)

@HiltViewModel
class PropiedadesViewModel
@Inject
constructor(
    private val repository: PropiedadesRepository,
    private val networkMonitor: ConnectivityObserver,
    private val apiService: PropiedadesApiService,
) : ViewModel() {
    private val _filters = MutableStateFlow(PropiedadesFilterState())
    val filters: StateFlow<PropiedadesFilterState> = _filters.asStateFlow()

    val propiedades: StateFlow<PropiedadesUiState> =
        MutableStateFlow<PropiedadesUiState>(PropiedadesUiState.Loading).also { state ->
            viewModelScope.launch {
                _filters
                    .flatMapLatest { f ->
                        repository.observeFiltered(f.ciudad, f.estado, f.tipoPropiedad)
                    }
                    .collect { list -> state.value = PropiedadesUiState.Success(list) }
            }
        }

    private val _formState = MutableStateFlow(PropiedadFormState())
    val formState: StateFlow<PropiedadFormState> = _formState.asStateFlow()

    private val _detailState =
        MutableStateFlow<PropiedadDetailUiState>(PropiedadDetailUiState.Loading)
    val detailState: StateFlow<PropiedadDetailUiState> = _detailState.asStateFlow()

    private val _successMessage = MutableStateFlow<String?>(null)
    val successMessage: StateFlow<String?> = _successMessage.asStateFlow()

    private val _deleteTarget = MutableStateFlow<Propiedad?>(null)
    val deleteTarget: StateFlow<Propiedad?> = _deleteTarget.asStateFlow()

    val isOnline: StateFlow<Boolean> =
        networkMonitor.isOnline.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    private var editingId: String? = null

    fun updateFilter(
        ciudad: String? = null,
        estado: String? = null,
        tipoPropiedad: String? = null,
    ) {
        _filters.update { it.copy(ciudad = ciudad, estado = estado, tipoPropiedad = tipoPropiedad) }
    }

    fun clearFilters() {
        _filters.value = PropiedadesFilterState()
    }

    fun loadDetail(id: String) {
        viewModelScope.launch {
            repository.observeById(id).collect { propiedad ->
                _detailState.value =
                    if (propiedad != null) {
                        PropiedadDetailUiState.Success(propiedad)
                    } else {
                        PropiedadDetailUiState.NotFound("Propiedad no encontrada")
                    }
            }
        }
    }

    fun initCreateForm() {
        editingId = null
        _formState.value = PropiedadFormState()
    }

    fun initEditForm(propiedad: Propiedad) {
        editingId = propiedad.id
        _formState.value =
            PropiedadFormState(
                titulo = propiedad.titulo,
                descripcion = propiedad.descripcion ?: "",
                direccion = propiedad.direccion,
                ciudad = propiedad.ciudad,
                provincia = propiedad.provincia,
                tipoPropiedad = propiedad.tipoPropiedad,
                habitaciones = propiedad.habitaciones?.toString() ?: "",
                banos = propiedad.banos?.toString() ?: "",
                areaM2 = propiedad.areaM2?.toPlainString() ?: "",
                precio = propiedad.precio.toPlainString(),
                moneda = propiedad.moneda,
                estado = propiedad.estado,
            )
    }

    fun onFieldChange(field: String, value: String) {
        _formState.update { state ->
            val newErrors = state.errors - field
            when (field) {
                "titulo" -> state.copy(titulo = value, errors = newErrors)
                "descripcion" -> state.copy(descripcion = value, errors = newErrors)
                "direccion" -> state.copy(direccion = value, errors = newErrors)
                "ciudad" -> state.copy(ciudad = value, errors = newErrors)
                "provincia" -> state.copy(provincia = value, errors = newErrors)
                "tipoPropiedad" -> state.copy(tipoPropiedad = value, errors = newErrors)
                "habitaciones" -> state.copy(habitaciones = value, errors = newErrors)
                "banos" -> state.copy(banos = value, errors = newErrors)
                "areaM2" -> state.copy(areaM2 = value, errors = newErrors)
                "precio" -> state.copy(precio = value, errors = newErrors)
                "moneda" -> state.copy(moneda = value, errors = newErrors)
                "estado" -> state.copy(estado = value, errors = newErrors)
                else -> state
            }
        }
    }

    fun save(onSuccess: () -> Unit) {
        val form = _formState.value
        val validation =
            PropiedadValidator.validateCreate(
                titulo = form.titulo,
                direccion = form.direccion,
                ciudad = form.ciudad,
                provincia = form.provincia,
                tipoPropiedad = form.tipoPropiedad,
                precio = form.precio,
            )
        val errors =
            validation
                .filterValues { it is ValidationResult.Invalid }
                .mapValues { (it.value as ValidationResult.Invalid).message }

        if (errors.isNotEmpty()) {
            _formState.update { it.copy(errors = errors) }
            return
        }

        viewModelScope.launch {
            _formState.update { it.copy(isSubmitting = true) }
            val result =
                if (editingId != null) {
                    repository.update(
                        editingId.orEmpty(),
                        UpdatePropiedadRequest(
                            titulo = form.titulo,
                            descripcion = form.descripcion.ifBlank { null },
                            direccion = form.direccion,
                            ciudad = form.ciudad,
                            provincia = form.provincia,
                            tipoPropiedad = form.tipoPropiedad,
                            habitaciones = form.habitaciones.toIntOrNull(),
                            banos = form.banos.toIntOrNull(),
                            areaM2 = form.areaM2.ifBlank { null },
                            precio = form.precio,
                            moneda = form.moneda,
                            estado = form.estado,
                        ),
                    )
                } else {
                    repository
                        .create(
                            CreatePropiedadRequest(
                                titulo = form.titulo,
                                descripcion = form.descripcion.ifBlank { null },
                                direccion = form.direccion,
                                ciudad = form.ciudad,
                                provincia = form.provincia,
                                tipoPropiedad = form.tipoPropiedad,
                                habitaciones = form.habitaciones.toIntOrNull(),
                                banos = form.banos.toIntOrNull(),
                                areaM2 = form.areaM2.ifBlank { null },
                                precio = form.precio,
                                moneda = form.moneda,
                                estado = form.estado,
                            )
                        )
                        .map {}
                }
            _formState.update { it.copy(isSubmitting = false) }
            result
                .onSuccess {
                    _successMessage.value =
                        if (editingId != null) "Actualizado correctamente"
                        else "Creado correctamente"
                    onSuccess()
                }
                .onFailure { e ->
                    _formState.update {
                        it.copy(errors = mapOf("general" to (e.message ?: "Error desconocido")))
                    }
                }
        }
    }

    fun requestDelete(propiedad: Propiedad) {
        _deleteTarget.value = propiedad
    }

    fun dismissDelete() {
        _deleteTarget.value = null
    }

    fun confirmDelete() {
        val target = _deleteTarget.value ?: return
        viewModelScope.launch {
            repository.delete(target.id)
            _deleteTarget.value = null
            _successMessage.value = "Eliminado correctamente"
        }
    }

    fun clearSuccessMessage() {
        _successMessage.value = null
    }

    // --- Unidades ---

    private val _unidadesState = MutableStateFlow<UnidadesUiState>(UnidadesUiState.Loading)
    val unidadesState: StateFlow<UnidadesUiState> = _unidadesState.asStateFlow()

    private val _unidadFormState = MutableStateFlow(UnidadFormState())
    val unidadFormState: StateFlow<UnidadFormState> = _unidadFormState.asStateFlow()

    private val _unidadDeleteTarget = MutableStateFlow<Unidad?>(null)
    val unidadDeleteTarget: StateFlow<Unidad?> = _unidadDeleteTarget.asStateFlow()

    private var editingUnidadId: String? = null
    private var currentPropiedadId: String? = null

    fun loadUnidades(propiedadId: String) {
        currentPropiedadId = propiedadId
        viewModelScope.launch {
            _unidadesState.value = UnidadesUiState.Loading
            try {
                val response = apiService.getUnidades(propiedadId)
                if (response.isSuccessful) {
                    val unidades = response.body().orEmpty().map { dto ->
                        Unidad(
                            id = dto.id,
                            propiedadId = dto.propiedadId,
                            numeroUnidad = dto.numeroUnidad,
                            piso = dto.piso,
                            habitaciones = dto.habitaciones,
                            banos = dto.banos,
                            areaM2 = dto.areaM2?.let { BigDecimal(it) },
                            precio = BigDecimal(dto.precio),
                            moneda = dto.moneda,
                            estado = dto.estado,
                            descripcion = dto.descripcion,
                            createdAt = Instant.parse(dto.createdAt),
                            updatedAt = Instant.parse(dto.updatedAt),
                        )
                    }
                    _unidadesState.value = UnidadesUiState.Success(unidades.toImmutableList())
                } else {
                    _unidadesState.value = UnidadesUiState.Error("Error al cargar unidades")
                }
            } catch (e: Exception) {
                _unidadesState.value = UnidadesUiState.Error(e.message ?: "Error desconocido")
            }
        }
    }

    fun initCreateUnidadForm() {
        editingUnidadId = null
        _unidadFormState.value = UnidadFormState()
    }

    fun initEditUnidadForm(unidad: Unidad) {
        editingUnidadId = unidad.id
        _unidadFormState.value = UnidadFormState(
            numeroUnidad = unidad.numeroUnidad,
            piso = unidad.piso?.toString() ?: "",
            habitaciones = unidad.habitaciones?.toString() ?: "",
            banos = unidad.banos?.toString() ?: "",
            areaM2 = unidad.areaM2?.toPlainString() ?: "",
            precio = unidad.precio.toPlainString(),
            moneda = unidad.moneda,
            estado = unidad.estado,
            descripcion = unidad.descripcion ?: "",
        )
    }

    fun onUnidadFieldChange(field: String, value: String) {
        _unidadFormState.update { state ->
            val newErrors = state.errors - field
            when (field) {
                "numeroUnidad" -> state.copy(numeroUnidad = value, errors = newErrors)
                "piso" -> state.copy(piso = value, errors = newErrors)
                "habitaciones" -> state.copy(habitaciones = value, errors = newErrors)
                "banos" -> state.copy(banos = value, errors = newErrors)
                "areaM2" -> state.copy(areaM2 = value, errors = newErrors)
                "precio" -> state.copy(precio = value, errors = newErrors)
                "moneda" -> state.copy(moneda = value, errors = newErrors)
                "estado" -> state.copy(estado = value, errors = newErrors)
                "descripcion" -> state.copy(descripcion = value, errors = newErrors)
                else -> state
            }
        }
    }

    fun saveUnidad(onSuccess: () -> Unit) {
        val form = _unidadFormState.value
        val errors = mutableMapOf<String, String>()
        if (form.numeroUnidad.isBlank()) errors["numeroUnidad"] = "Este campo es requerido"
        if (form.precio.isBlank()) {
            errors["precio"] = "Este campo es requerido"
        } else {
            try {
                if (BigDecimal(form.precio) <= BigDecimal.ZERO) {
                    errors["precio"] = "El valor debe ser mayor a cero"
                }
            } catch (_: NumberFormatException) {
                errors["precio"] = "Valor numérico inválido"
            }
        }

        if (errors.isNotEmpty()) {
            _unidadFormState.update { it.copy(errors = errors) }
            return
        }

        val propiedadId = currentPropiedadId ?: return

        viewModelScope.launch {
            _unidadFormState.update { it.copy(isSubmitting = true) }
            try {
                val result = if (editingUnidadId != null) {
                    apiService.updateUnidad(
                        propiedadId = propiedadId,
                        unidadId = editingUnidadId.orEmpty(),
                        request = UpdateUnidadRequest(
                            numeroUnidad = form.numeroUnidad,
                            piso = form.piso.toIntOrNull(),
                            habitaciones = form.habitaciones.toIntOrNull(),
                            banos = form.banos.toIntOrNull(),
                            areaM2 = form.areaM2.ifBlank { null },
                            precio = form.precio,
                            moneda = form.moneda,
                            estado = form.estado,
                            descripcion = form.descripcion.ifBlank { null },
                        ),
                    )
                } else {
                    apiService.createUnidad(
                        propiedadId = propiedadId,
                        request = CreateUnidadRequest(
                            numeroUnidad = form.numeroUnidad,
                            piso = form.piso.toIntOrNull(),
                            habitaciones = form.habitaciones.toIntOrNull(),
                            banos = form.banos.toIntOrNull(),
                            areaM2 = form.areaM2.ifBlank { null },
                            precio = form.precio,
                            moneda = form.moneda.ifBlank { null },
                            estado = form.estado.ifBlank { null },
                            descripcion = form.descripcion.ifBlank { null },
                        ),
                    )
                }
                _unidadFormState.update { it.copy(isSubmitting = false) }
                if (result.isSuccessful) {
                    _successMessage.value = if (editingUnidadId != null) "Unidad actualizada" else "Unidad creada"
                    loadUnidades(propiedadId)
                    onSuccess()
                } else {
                    _unidadFormState.update {
                        it.copy(errors = mapOf("general" to "Error al guardar unidad"))
                    }
                }
            } catch (e: Exception) {
                _unidadFormState.update {
                    it.copy(
                        isSubmitting = false,
                        errors = mapOf("general" to (e.message ?: "Error desconocido")),
                    )
                }
            }
        }
    }

    fun requestDeleteUnidad(unidad: Unidad) {
        _unidadDeleteTarget.value = unidad
    }

    fun dismissDeleteUnidad() {
        _unidadDeleteTarget.value = null
    }

    fun confirmDeleteUnidad() {
        val target = _unidadDeleteTarget.value ?: return
        val propiedadId = currentPropiedadId ?: return
        viewModelScope.launch {
            try {
                val response = apiService.deleteUnidad(propiedadId, target.id)
                if (response.isSuccessful) {
                    _successMessage.value = "Unidad eliminada"
                    loadUnidades(propiedadId)
                }
            } catch (_: Exception) {
                // Silently handle - user can retry
            }
            _unidadDeleteTarget.value = null
        }
    }
}
