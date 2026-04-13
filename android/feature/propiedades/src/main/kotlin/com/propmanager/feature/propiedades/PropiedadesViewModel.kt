package com.propmanager.feature.propiedades

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.common.PropiedadValidator
import com.propmanager.core.data.repository.PropiedadesRepository
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.ValidationResult
import com.propmanager.core.model.dto.CreatePropiedadRequest
import com.propmanager.core.model.dto.UpdatePropiedadRequest
import com.propmanager.core.network.ConnectivityObserver
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
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

@HiltViewModel
class PropiedadesViewModel
@Inject
constructor(
    private val repository: PropiedadesRepository,
    private val networkMonitor: ConnectivityObserver,
) : ViewModel() {
    private val _filters = MutableStateFlow(PropiedadesFilterState())
    val filters: StateFlow<PropiedadesFilterState> = _filters.asStateFlow()

    val propiedades: StateFlow<PropiedadesUiState> =
        _filters
            .flatMapLatest { f -> repository.observeFiltered(f.ciudad, f.estado, f.tipoPropiedad) }
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), emptyList())
            .let { flow ->
                MutableStateFlow<PropiedadesUiState>(PropiedadesUiState.Loading).also { state ->
                    viewModelScope.launch {
                        _filters
                            .flatMapLatest { f ->
                                repository.observeFiltered(f.ciudad, f.estado, f.tipoPropiedad)
                            }
                            .collect { list -> state.value = PropiedadesUiState.Success(list) }
                    }
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
                        editingId!!,
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
}
