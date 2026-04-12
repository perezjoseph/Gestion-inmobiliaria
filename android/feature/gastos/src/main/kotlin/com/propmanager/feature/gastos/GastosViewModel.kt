package com.propmanager.feature.gastos

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.common.DateFormatter
import com.propmanager.core.common.GastoValidator
import com.propmanager.core.data.repository.GastosRepository
import com.propmanager.core.data.repository.PropiedadesRepository
import com.propmanager.core.model.Gasto
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.ValidationResult
import com.propmanager.core.model.dto.CreateGastoRequest
import com.propmanager.core.model.dto.UpdateGastoRequest
import com.propmanager.core.network.NetworkMonitor
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.flatMapLatest
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.time.LocalDate
import javax.inject.Inject

data class GastosFilterState(
    val propiedadId: String? = null,
    val categoria: String? = null,
    val estado: String? = null,
    val fechaDesde: String? = null,
    val fechaHasta: String? = null,
)

data class GastoFormState(
    val propiedadId: String = "",
    val categoria: String = "",
    val descripcion: String = "",
    val monto: String = "",
    val moneda: String = "DOP",
    val fechaGasto: LocalDate? = null,
    val estado: String = "pendiente",
    val proveedor: String = "",
    val numeroFactura: String = "",
    val notas: String = "",
    val errors: Map<String, String> = emptyMap(),
    val isSubmitting: Boolean = false,
)

sealed interface GastosUiState {
    data object Loading : GastosUiState

    data class Success(
        val gastos: List<Gasto>,
    ) : GastosUiState
}

@OptIn(ExperimentalCoroutinesApi::class)
@HiltViewModel
class GastosViewModel
    @Inject
    constructor(
        private val gastosRepository: GastosRepository,
        private val propiedadesRepository: PropiedadesRepository,
        private val networkMonitor: NetworkMonitor,
    ) : ViewModel() {
        private val _filters = MutableStateFlow(GastosFilterState())
        val filters: StateFlow<GastosFilterState> = _filters.asStateFlow()

        val gastos: StateFlow<GastosUiState> =
            MutableStateFlow<GastosUiState>(GastosUiState.Loading).also { state ->
                viewModelScope.launch {
                    _filters
                        .flatMapLatest { f ->
                            gastosRepository.observeFiltered(f.propiedadId, f.categoria, f.estado, f.fechaDesde, f.fechaHasta)
                        }.collect { state.value = GastosUiState.Success(it) }
                }
            }

        private val _formState = MutableStateFlow(GastoFormState())
        val formState: StateFlow<GastoFormState> = _formState.asStateFlow()

        private val _successMessage = MutableStateFlow<String?>(null)
        val successMessage: StateFlow<String?> = _successMessage.asStateFlow()

        private val _deleteTarget = MutableStateFlow<Gasto?>(null)
        val deleteTarget: StateFlow<Gasto?> = _deleteTarget.asStateFlow()

        val propiedades: StateFlow<List<Propiedad>> =
            propiedadesRepository
                .observeAll()
                .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), emptyList())

        val isOnline: StateFlow<Boolean> =
            networkMonitor.isOnline
                .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

        private var editingId: String? = null

        fun updateFilter(
            propiedadId: String? = null,
            categoria: String? = null,
            estado: String? = null,
        ) {
            _filters.update { it.copy(propiedadId = propiedadId, categoria = categoria, estado = estado) }
        }

        fun clearFilters() {
            _filters.value = GastosFilterState()
        }

        fun initCreateForm() {
            editingId = null
            _formState.value = GastoFormState()
        }

        fun initEditForm(gasto: Gasto) {
            editingId = gasto.id
            _formState.value =
                GastoFormState(
                    propiedadId = gasto.propiedadId,
                    categoria = gasto.categoria,
                    descripcion = gasto.descripcion,
                    monto = gasto.monto.toPlainString(),
                    moneda = gasto.moneda,
                    fechaGasto = gasto.fechaGasto,
                    estado = gasto.estado,
                    proveedor = gasto.proveedor ?: "",
                    numeroFactura = gasto.numeroFactura ?: "",
                    notas = gasto.notas ?: "",
                )
        }

        fun prefillFromOcr(
            monto: String?,
            fecha: LocalDate?,
            proveedor: String?,
            numeroFactura: String?,
        ) {
            _formState.update {
                it.copy(
                    monto = monto ?: it.monto,
                    fechaGasto = fecha ?: it.fechaGasto,
                    proveedor = proveedor ?: it.proveedor,
                    numeroFactura = numeroFactura ?: it.numeroFactura,
                )
            }
        }

        fun onFieldChange(
            field: String,
            value: String,
        ) {
            _formState.update { state ->
                val newErrors = state.errors - field
                when (field) {
                    "propiedadId" -> state.copy(propiedadId = value, errors = newErrors)
                    "categoria" -> state.copy(categoria = value, errors = newErrors)
                    "descripcion" -> state.copy(descripcion = value, errors = newErrors)
                    "monto" -> state.copy(monto = value, errors = newErrors)
                    "moneda" -> state.copy(moneda = value, errors = newErrors)
                    "proveedor" -> state.copy(proveedor = value, errors = newErrors)
                    "numeroFactura" -> state.copy(numeroFactura = value, errors = newErrors)
                    "notas" -> state.copy(notas = value, errors = newErrors)
                    else -> state
                }
            }
        }

        fun onFechaGastoChange(date: LocalDate) {
            _formState.update { it.copy(fechaGasto = date, errors = it.errors - "fechaGasto") }
        }

        fun save(onSuccess: () -> Unit) {
            val form = _formState.value
            val fechaStr = form.fechaGasto?.let { DateFormatter.toApi(it) } ?: ""
            val validation =
                GastoValidator.validateCreate(
                    form.propiedadId,
                    form.categoria,
                    form.descripcion,
                    form.monto,
                    form.moneda,
                    fechaStr,
                )
            val errors =
                validation.filterValues { it is ValidationResult.Invalid }.mapValues {
                    (it.value as ValidationResult.Invalid)
                        .message
                }
            if (errors.isNotEmpty()) {
                _formState.update { it.copy(errors = errors) }
                return
            }

            viewModelScope.launch {
                _formState.update { it.copy(isSubmitting = true) }
                val result =
                    if (editingId != null) {
                        gastosRepository.update(
                            editingId!!,
                            UpdateGastoRequest(
                                categoria = form.categoria,
                                descripcion = form.descripcion,
                                monto = form.monto,
                                moneda = form.moneda,
                                fechaGasto = fechaStr,
                                proveedor = form.proveedor.ifBlank { null },
                                numeroFactura = form.numeroFactura.ifBlank { null },
                                notas = form.notas.ifBlank { null },
                            ),
                        )
                    } else {
                        gastosRepository
                            .create(
                                CreateGastoRequest(
                                    propiedadId = form.propiedadId,
                                    categoria = form.categoria,
                                    descripcion = form.descripcion,
                                    monto = form.monto,
                                    moneda = form.moneda,
                                    fechaGasto = fechaStr,
                                    proveedor = form.proveedor.ifBlank { null },
                                    numeroFactura = form.numeroFactura.ifBlank { null },
                                    notas = form.notas.ifBlank { null },
                                ),
                            ).map { }
                    }
                _formState.update { it.copy(isSubmitting = false) }
                result
                    .onSuccess {
                        _successMessage.value =
                            if (editingId !=
                                null
                            ) {
                                "Actualizado correctamente"
                            } else {
                                "Creado correctamente"
                            }
                        ; onSuccess()
                    }.onFailure { e -> _formState.update { it.copy(errors = mapOf("general" to (e.message ?: "Error desconocido"))) } }
            }
        }

        fun requestDelete(gasto: Gasto) {
            _deleteTarget.value = gasto
        }

        fun dismissDelete() {
            _deleteTarget.value = null
        }

        fun confirmDelete() {
            val target = _deleteTarget.value ?: return
            viewModelScope.launch {
                gastosRepository.delete(target.id)
                _deleteTarget.value = null
                _successMessage.value =
                    "Eliminado correctamente"
            }
        }

        fun clearSuccessMessage() {
            _successMessage.value = null
        }
    }
