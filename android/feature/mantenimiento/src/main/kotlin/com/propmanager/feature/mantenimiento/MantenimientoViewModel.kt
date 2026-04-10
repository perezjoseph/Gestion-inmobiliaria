package com.propmanager.feature.mantenimiento

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.common.SolicitudValidator
import com.propmanager.core.data.repository.MantenimientoRepository
import com.propmanager.core.data.repository.PropiedadesRepository
import com.propmanager.core.model.NotaMantenimiento
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.SolicitudMantenimiento
import com.propmanager.core.model.ValidationResult
import com.propmanager.core.model.dto.CreateNotaRequest
import com.propmanager.core.model.dto.CreateSolicitudRequest
import com.propmanager.core.model.dto.UpdateEstadoRequest
import com.propmanager.core.model.dto.UpdateSolicitudRequest
import com.propmanager.core.network.ConnectivityObserver
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class MantenimientoFilterState(
    val estado: String? = null,
    val prioridad: String? = null,
    val propiedadId: String? = null,
)

data class SolicitudFormState(
    val propiedadId: String = "",
    val titulo: String = "",
    val descripcion: String = "",
    val prioridad: String = "media",
    val nombreProveedor: String = "",
    val telefonoProveedor: String = "",
    val emailProveedor: String = "",
    val costoMonto: String = "",
    val costoMoneda: String = "DOP",
    val errors: Map<String, String> = emptyMap(),
    val isSubmitting: Boolean = false,
)

sealed interface MantenimientoUiState {
    data object Loading : MantenimientoUiState

    data class Success(val solicitudes: List<SolicitudMantenimiento>) : MantenimientoUiState
}

sealed interface SolicitudDetailUiState {
    data object Loading : SolicitudDetailUiState

    data class Success(val solicitud: SolicitudMantenimiento, val notas: List<NotaMantenimiento>) :
        SolicitudDetailUiState

    data class NotFound(val message: String) : SolicitudDetailUiState
}

@OptIn(ExperimentalCoroutinesApi::class)
@HiltViewModel
class MantenimientoViewModel
@Inject
constructor(
    private val mantenimientoRepository: MantenimientoRepository,
    private val propiedadesRepository: PropiedadesRepository,
    private val networkMonitor: ConnectivityObserver,
) : ViewModel() {
    private val _filters = MutableStateFlow(MantenimientoFilterState())
    val filters: StateFlow<MantenimientoFilterState> = _filters.asStateFlow()

    val solicitudes: StateFlow<MantenimientoUiState> =
        MutableStateFlow<MantenimientoUiState>(MantenimientoUiState.Loading).also { state ->
            viewModelScope.launch {
                _filters
                    .flatMapLatest { f ->
                        mantenimientoRepository.observeFiltered(
                            f.estado,
                            f.prioridad,
                            f.propiedadId,
                        )
                    }
                    .collect { state.value = MantenimientoUiState.Success(it) }
            }
        }

    private val _detailState =
        MutableStateFlow<SolicitudDetailUiState>(SolicitudDetailUiState.Loading)
    val detailState: StateFlow<SolicitudDetailUiState> = _detailState.asStateFlow()

    private val _formState = MutableStateFlow(SolicitudFormState())
    val formState: StateFlow<SolicitudFormState> = _formState.asStateFlow()

    private val _notaInput = MutableStateFlow("")
    val notaInput: StateFlow<String> = _notaInput.asStateFlow()

    private val _successMessage = MutableStateFlow<String?>(null)
    val successMessage: StateFlow<String?> = _successMessage.asStateFlow()

    private val _deleteTarget = MutableStateFlow<SolicitudMantenimiento?>(null)
    val deleteTarget: StateFlow<SolicitudMantenimiento?> = _deleteTarget.asStateFlow()

    private val _showEstadoDialog = MutableStateFlow(false)
    val showEstadoDialog: StateFlow<Boolean> = _showEstadoDialog.asStateFlow()

    val propiedades: StateFlow<List<Propiedad>> =
        propiedadesRepository
            .observeAll()
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), emptyList())

    val isOnline: StateFlow<Boolean> =
        networkMonitor.isOnline.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    private var editingId: String? = null
    private var currentSolicitudId: String? = null

    fun updateFilter(
        estado: String? = null,
        prioridad: String? = null,
        propiedadId: String? = null,
    ) {
        _filters.update {
            it.copy(estado = estado, prioridad = prioridad, propiedadId = propiedadId)
        }
    }

    fun clearFilters() {
        _filters.value = MantenimientoFilterState()
    }

    fun loadDetail(id: String) {
        currentSolicitudId = id
        viewModelScope.launch {
            mantenimientoRepository.observeById(id).collect { solicitud ->
                if (solicitud == null) {
                    _detailState.value = SolicitudDetailUiState.NotFound("Solicitud no encontrada")
                } else {
                    mantenimientoRepository.observeNotas(id).collect { notas ->
                        _detailState.value = SolicitudDetailUiState.Success(solicitud, notas)
                    }
                }
            }
        }
    }

    fun initCreateForm() {
        editingId = null
        _formState.value = SolicitudFormState()
    }

    fun initEditForm(solicitud: SolicitudMantenimiento) {
        editingId = solicitud.id
        _formState.value =
            SolicitudFormState(
                propiedadId = solicitud.propiedadId,
                titulo = solicitud.titulo,
                descripcion = solicitud.descripcion ?: "",
                prioridad = solicitud.prioridad,
                nombreProveedor = solicitud.nombreProveedor ?: "",
                telefonoProveedor = solicitud.telefonoProveedor ?: "",
                emailProveedor = solicitud.emailProveedor ?: "",
                costoMonto = solicitud.costoMonto?.toPlainString() ?: "",
                costoMoneda = solicitud.costoMoneda ?: "DOP",
            )
    }

    fun onFieldChange(field: String, value: String) {
        _formState.update { state ->
            val newErrors = state.errors - field
            when (field) {
                "propiedadId" -> state.copy(propiedadId = value, errors = newErrors)
                "titulo" -> state.copy(titulo = value, errors = newErrors)
                "descripcion" -> state.copy(descripcion = value, errors = newErrors)
                "prioridad" -> state.copy(prioridad = value, errors = newErrors)
                "nombreProveedor" -> state.copy(nombreProveedor = value, errors = newErrors)
                "telefonoProveedor" -> state.copy(telefonoProveedor = value, errors = newErrors)
                "emailProveedor" -> state.copy(emailProveedor = value, errors = newErrors)
                "costoMonto" -> state.copy(costoMonto = value, errors = newErrors)
                "costoMoneda" -> state.copy(costoMoneda = value, errors = newErrors)
                else -> state
            }
        }
    }

    fun save(onSuccess: () -> Unit) {
        val form = _formState.value
        val validation = SolicitudValidator.validateCreate(form.propiedadId, form.titulo)
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
                    mantenimientoRepository.update(
                        editingId!!,
                        UpdateSolicitudRequest(
                            titulo = form.titulo,
                            descripcion = form.descripcion.ifBlank { null },
                            prioridad = form.prioridad,
                            nombreProveedor = form.nombreProveedor.ifBlank { null },
                            telefonoProveedor = form.telefonoProveedor.ifBlank { null },
                            emailProveedor = form.emailProveedor.ifBlank { null },
                            costoMonto = form.costoMonto.ifBlank { null },
                            costoMoneda = form.costoMoneda.ifBlank { null },
                        ),
                    )
                } else {
                    mantenimientoRepository
                        .create(
                            CreateSolicitudRequest(
                                propiedadId = form.propiedadId,
                                titulo = form.titulo,
                                descripcion = form.descripcion.ifBlank { null },
                                prioridad = form.prioridad,
                                nombreProveedor = form.nombreProveedor.ifBlank { null },
                                telefonoProveedor = form.telefonoProveedor.ifBlank { null },
                                emailProveedor = form.emailProveedor.ifBlank { null },
                                costoMonto = form.costoMonto.ifBlank { null },
                                costoMoneda = form.costoMoneda.ifBlank { null },
                            )
                        )
                        .map {}
                }
            _formState.update { it.copy(isSubmitting = false) }
            result
                .onSuccess {
                    _successMessage.value =
                        if (editingId != null) {
                            "Actualizado correctamente"
                        } else {
                            "Creado correctamente"
                        }

                    onSuccess()
                }
                .onFailure { e ->
                    _formState.update {
                        it.copy(errors = mapOf("general" to (e.message ?: "Error desconocido")))
                    }
                }
        }
    }

    fun onNotaInputChange(value: String) {
        _notaInput.value = value
    }

    fun addNota() {
        val id = currentSolicitudId ?: return
        val contenido = _notaInput.value.trim()
        if (contenido.isBlank()) return
        viewModelScope.launch {
            mantenimientoRepository.addNota(id, CreateNotaRequest(contenido))
            _notaInput.value = ""
            _successMessage.value = "Nota agregada"
        }
    }

    fun showEstadoChange() {
        _showEstadoDialog.value = true
    }

    fun dismissEstadoChange() {
        _showEstadoDialog.value = false
    }

    fun changeEstado(nuevoEstado: String) {
        val id = currentSolicitudId ?: return
        viewModelScope.launch {
            mantenimientoRepository.updateEstado(id, UpdateEstadoRequest(nuevoEstado))
            _showEstadoDialog.value = false
            _successMessage.value = "Estado actualizado"
        }
    }

    fun requestDelete(solicitud: SolicitudMantenimiento) {
        _deleteTarget.value = solicitud
    }

    fun dismissDelete() {
        _deleteTarget.value = null
    }

    fun confirmDelete() {
        val target = _deleteTarget.value ?: return
        viewModelScope.launch {
            mantenimientoRepository.delete(target.id)
            _deleteTarget.value = null
            _successMessage.value = "Eliminado correctamente"
        }
    }

    fun clearSuccessMessage() {
        _successMessage.value = null
    }
}
