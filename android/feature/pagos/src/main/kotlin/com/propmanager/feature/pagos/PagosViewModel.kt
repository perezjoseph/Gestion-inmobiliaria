package com.propmanager.feature.pagos

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.common.DateFormatter
import com.propmanager.core.common.PagoValidator
import com.propmanager.core.data.repository.ContratosRepository
import com.propmanager.core.data.repository.PagosRepository
import com.propmanager.core.model.Contrato
import com.propmanager.core.model.Pago
import com.propmanager.core.model.ValidationResult
import com.propmanager.core.model.dto.CreatePagoRequest
import com.propmanager.core.model.dto.UpdatePagoRequest
import com.propmanager.core.network.ConnectivityObserver
import dagger.hilt.android.lifecycle.HiltViewModel
import java.time.LocalDate
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

data class PagosFilterState(
    val contratoId: String? = null,
    val estado: String? = null,
    val fechaDesde: String? = null,
    val fechaHasta: String? = null,
)

data class PagoFormState(
    val contratoId: String = "",
    val monto: String = "",
    val moneda: String = "DOP",
    val fechaPago: LocalDate? = null,
    val fechaVencimiento: LocalDate? = null,
    val metodoPago: String = "",
    val notas: String = "",
    val errors: Map<String, String> = emptyMap(),
    val isSubmitting: Boolean = false,
)

sealed interface PagosUiState {
    data object Loading : PagosUiState

    data class Success(val pagos: List<Pago>) : PagosUiState
}

@OptIn(ExperimentalCoroutinesApi::class)
@HiltViewModel
class PagosViewModel
@Inject
constructor(
    private val pagosRepository: PagosRepository,
    private val contratosRepository: ContratosRepository,
    private val networkMonitor: ConnectivityObserver,
) : ViewModel() {
    private val _filters = MutableStateFlow(PagosFilterState())
    val filters: StateFlow<PagosFilterState> = _filters.asStateFlow()

    val pagos: StateFlow<PagosUiState> =
        MutableStateFlow<PagosUiState>(PagosUiState.Loading).also { state ->
            viewModelScope.launch {
                _filters
                    .flatMapLatest { f ->
                        pagosRepository.observeFiltered(
                            f.contratoId,
                            f.estado,
                            f.fechaDesde,
                            f.fechaHasta,
                        )
                    }
                    .collect { state.value = PagosUiState.Success(it) }
            }
        }

    private val _formState = MutableStateFlow(PagoFormState())
    val formState: StateFlow<PagoFormState> = _formState.asStateFlow()

    private val _successMessage = MutableStateFlow<String?>(null)
    val successMessage: StateFlow<String?> = _successMessage.asStateFlow()

    private val _deleteTarget = MutableStateFlow<Pago?>(null)
    val deleteTarget: StateFlow<Pago?> = _deleteTarget.asStateFlow()

    val contratos: StateFlow<List<Contrato>> =
        contratosRepository
            .observeAll()
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), emptyList())

    val isOnline: StateFlow<Boolean> =
        networkMonitor.isOnline.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    private var editingId: String? = null

    fun updateFilter(
        contratoId: String? = null,
        estado: String? = null,
        fechaDesde: String? = null,
        fechaHasta: String? = null,
    ) {
        _filters.update {
            it.copy(
                contratoId = contratoId,
                estado = estado,
                fechaDesde = fechaDesde,
                fechaHasta = fechaHasta,
            )
        }
    }

    fun clearFilters() {
        _filters.value = PagosFilterState()
    }

    fun initCreateForm() {
        editingId = null
        _formState.value = PagoFormState()
    }

    fun initEditForm(pago: Pago) {
        editingId = pago.id
        _formState.value =
            PagoFormState(
                contratoId = pago.contratoId,
                monto = pago.monto.toPlainString(),
                moneda = pago.moneda,
                fechaPago = pago.fechaPago,
                fechaVencimiento = pago.fechaVencimiento,
                metodoPago = pago.metodoPago ?: "",
                notas = pago.notas ?: "",
            )
    }

    fun onFieldChange(field: String, value: String) {
        _formState.update { state ->
            val newErrors = state.errors - field
            when (field) {
                "contratoId" -> state.copy(contratoId = value, errors = newErrors)
                "monto" -> state.copy(monto = value, errors = newErrors)
                "moneda" -> state.copy(moneda = value, errors = newErrors)
                "metodoPago" -> state.copy(metodoPago = value, errors = newErrors)
                "notas" -> state.copy(notas = value, errors = newErrors)
                else -> state
            }
        }
    }

    fun onFechaPagoChange(date: LocalDate) {
        _formState.update { it.copy(fechaPago = date) }
    }

    fun onFechaVencimientoChange(date: LocalDate) {
        _formState.update {
            it.copy(fechaVencimiento = date, errors = it.errors - "fechaVencimiento")
        }
    }

    fun save(onSuccess: () -> Unit) {
        val form = _formState.value
        val fechaVencStr = form.fechaVencimiento?.let { DateFormatter.toApi(it) } ?: ""
        val validation = PagoValidator.validateCreate(form.contratoId, form.monto, fechaVencStr)
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
                    pagosRepository.update(
                        editingId!!,
                        UpdatePagoRequest(
                            monto = form.monto,
                            fechaPago = form.fechaPago?.let { DateFormatter.toApi(it) },
                            metodoPago = form.metodoPago.ifBlank { null },
                            notas = form.notas.ifBlank { null },
                        ),
                    )
                } else {
                    pagosRepository
                        .create(
                            CreatePagoRequest(
                                contratoId = form.contratoId,
                                monto = form.monto,
                                moneda = form.moneda,
                                fechaPago = form.fechaPago?.let { DateFormatter.toApi(it) },
                                fechaVencimiento = fechaVencStr,
                                metodoPago = form.metodoPago.ifBlank { null },
                                notas = form.notas.ifBlank { null },
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

    fun requestDelete(pago: Pago) {
        _deleteTarget.value = pago
    }

    fun dismissDelete() {
        _deleteTarget.value = null
    }

    fun confirmDelete() {
        val target = _deleteTarget.value ?: return
        viewModelScope.launch {
            pagosRepository.delete(target.id)
            _deleteTarget.value = null
            _successMessage.value = "Eliminado correctamente"
        }
    }

    fun clearSuccessMessage() {
        _successMessage.value = null
    }
}
