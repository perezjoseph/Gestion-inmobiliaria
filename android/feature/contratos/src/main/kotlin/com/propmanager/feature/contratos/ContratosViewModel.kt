package com.propmanager.feature.contratos

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.common.ContratoValidator
import com.propmanager.core.common.DateFormatter
import com.propmanager.core.data.repository.ContratosRepository
import com.propmanager.core.data.repository.InquilinosRepository
import com.propmanager.core.data.repository.PropiedadesRepository
import com.propmanager.core.model.Contrato
import com.propmanager.core.model.Inquilino
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.ValidationResult
import com.propmanager.core.model.dto.CreateContratoRequest
import com.propmanager.core.model.dto.RenovarContratoRequest
import com.propmanager.core.model.dto.TerminarContratoRequest
import com.propmanager.core.network.NetworkMonitor
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.time.LocalDate
import javax.inject.Inject

data class ContratoWithNames(
    val contrato: Contrato,
    val propiedadTitulo: String,
    val inquilinoNombre: String,
)

data class ContratoFormState(
    val propiedadId: String = "",
    val inquilinoId: String = "",
    val fechaInicio: LocalDate? = null,
    val fechaFin: LocalDate? = null,
    val montoMensual: String = "",
    val deposito: String = "",
    val moneda: String = "DOP",
    val errors: Map<String, String> = emptyMap(),
    val isSubmitting: Boolean = false,
)

data class RenewFormState(
    val fechaFin: LocalDate? = null,
    val montoMensual: String = "",
    val errors: Map<String, String> = emptyMap(),
)

sealed interface ContratosUiState {
    data object Loading : ContratosUiState
    data class Success(val contratos: List<ContratoWithNames>) : ContratosUiState
}

sealed interface ContratoDetailUiState {
    data object Loading : ContratoDetailUiState
    data class Success(val contrato: ContratoWithNames) : ContratoDetailUiState
    data class NotFound(val message: String) : ContratoDetailUiState
}

@HiltViewModel
class ContratosViewModel @Inject constructor(
    private val contratosRepository: ContratosRepository,
    private val propiedadesRepository: PropiedadesRepository,
    private val inquilinosRepository: InquilinosRepository,
    private val networkMonitor: NetworkMonitor,
) : ViewModel() {

    val contratos: StateFlow<ContratosUiState> =
        MutableStateFlow<ContratosUiState>(ContratosUiState.Loading).also { state ->
            viewModelScope.launch {
                combine(
                    contratosRepository.observeAll(),
                    propiedadesRepository.observeAll(),
                    inquilinosRepository.observeAll(),
                ) { contratos, propiedades, inquilinos ->
                    val propMap = propiedades.associateBy { it.id }
                    val inqMap = inquilinos.associateBy { it.id }
                    contratos.map { c ->
                        ContratoWithNames(c, propMap[c.propiedadId]?.titulo ?: c.propiedadId, inqMap[c.inquilinoId]?.let { "${it.nombre} ${it.apellido}" } ?: c.inquilinoId)
                    }
                }.collect { state.value = ContratosUiState.Success(it) }
            }
        }

    val expiring: StateFlow<List<ContratoWithNames>> =
        MutableStateFlow<List<ContratoWithNames>>(emptyList()).also { state ->
            viewModelScope.launch {
                combine(
                    contratosRepository.observeExpiring(30),
                    propiedadesRepository.observeAll(),
                    inquilinosRepository.observeAll(),
                ) { contratos, propiedades, inquilinos ->
                    val propMap = propiedades.associateBy { it.id }
                    val inqMap = inquilinos.associateBy { it.id }
                    contratos.map { c ->
                        ContratoWithNames(c, propMap[c.propiedadId]?.titulo ?: c.propiedadId, inqMap[c.inquilinoId]?.let { "${it.nombre} ${it.apellido}" } ?: c.inquilinoId)
                    }
                }.collect { state.value = it }
            }
        }

    private val _detailState = MutableStateFlow<ContratoDetailUiState>(ContratoDetailUiState.Loading)
    val detailState: StateFlow<ContratoDetailUiState> = _detailState.asStateFlow()

    private val _formState = MutableStateFlow(ContratoFormState())
    val formState: StateFlow<ContratoFormState> = _formState.asStateFlow()

    private val _renewForm = MutableStateFlow(RenewFormState())
    val renewForm: StateFlow<RenewFormState> = _renewForm.asStateFlow()

    private val _showRenewDialog = MutableStateFlow(false)
    val showRenewDialog: StateFlow<Boolean> = _showRenewDialog.asStateFlow()

    private val _showTerminateDialog = MutableStateFlow(false)
    val showTerminateDialog: StateFlow<Boolean> = _showTerminateDialog.asStateFlow()

    private val _successMessage = MutableStateFlow<String?>(null)
    val successMessage: StateFlow<String?> = _successMessage.asStateFlow()

    private val _deleteTarget = MutableStateFlow<Contrato?>(null)
    val deleteTarget: StateFlow<Contrato?> = _deleteTarget.asStateFlow()

    val propiedades: StateFlow<List<Propiedad>> = propiedadesRepository.observeAll()
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), emptyList())

    val inquilinos: StateFlow<List<Inquilino>> = inquilinosRepository.observeAll()
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), emptyList())

    val isOnline: StateFlow<Boolean> = networkMonitor.isOnline
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    private var editingId: String? = null
    private var currentContratoId: String? = null

    fun loadDetail(id: String) {
        currentContratoId = id
        viewModelScope.launch {
            combine(
                contratosRepository.observeById(id),
                propiedadesRepository.observeAll(),
                inquilinosRepository.observeAll(),
            ) { contrato, propiedades, inquilinos ->
                if (contrato == null) return@combine null
                val propMap = propiedades.associateBy { it.id }
                val inqMap = inquilinos.associateBy { it.id }
                ContratoWithNames(contrato, propMap[contrato.propiedadId]?.titulo ?: contrato.propiedadId, inqMap[contrato.inquilinoId]?.let { "${it.nombre} ${it.apellido}" } ?: contrato.inquilinoId)
            }.collect { cwn ->
                _detailState.value = if (cwn != null) ContratoDetailUiState.Success(cwn) else ContratoDetailUiState.NotFound("Contrato no encontrado")
            }
        }
    }

    fun initCreateForm() {
        editingId = null
        _formState.value = ContratoFormState()
    }

    fun initEditForm(contrato: Contrato) {
        editingId = contrato.id
        _formState.value = ContratoFormState(
            propiedadId = contrato.propiedadId,
            inquilinoId = contrato.inquilinoId,
            fechaInicio = contrato.fechaInicio,
            fechaFin = contrato.fechaFin,
            montoMensual = contrato.montoMensual.toPlainString(),
            deposito = contrato.deposito?.toPlainString() ?: "",
            moneda = contrato.moneda,
        )
    }

    fun onFormFieldChange(field: String, value: String) {
        _formState.update { state ->
            val newErrors = state.errors - field
            when (field) {
                "propiedadId" -> state.copy(propiedadId = value, errors = newErrors)
                "inquilinoId" -> state.copy(inquilinoId = value, errors = newErrors)
                "montoMensual" -> state.copy(montoMensual = value, errors = newErrors)
                "deposito" -> state.copy(deposito = value, errors = newErrors)
                "moneda" -> state.copy(moneda = value, errors = newErrors)
                else -> state
            }
        }
    }

    fun onFechaInicioChange(date: LocalDate) {
        _formState.update { it.copy(fechaInicio = date, errors = it.errors - "fechaInicio") }
    }

    fun onFechaFinChange(date: LocalDate) {
        _formState.update { it.copy(fechaFin = date, errors = it.errors - "fechaFin") }
    }

    fun save(onSuccess: () -> Unit) {
        val form = _formState.value
        val fechaInicioStr = form.fechaInicio?.let { DateFormatter.toApi(it) } ?: ""
        val fechaFinStr = form.fechaFin?.let { DateFormatter.toApi(it) } ?: ""
        val validation = ContratoValidator.validateCreate(form.propiedadId, form.inquilinoId, fechaInicioStr, fechaFinStr, form.montoMensual)
        val errors = validation.filterValues { it is ValidationResult.Invalid }.mapValues { (it.value as ValidationResult.Invalid).message }

        if (errors.isNotEmpty()) { _formState.update { it.copy(errors = errors) }; return }

        viewModelScope.launch {
            _formState.update { it.copy(isSubmitting = true) }
            val result = if (editingId != null) {
                contratosRepository.delete(editingId!!).flatMapSuspend {
                    contratosRepository.create(CreateContratoRequest(
                        propiedadId = form.propiedadId, inquilinoId = form.inquilinoId,
                        fechaInicio = fechaInicioStr, fechaFin = fechaFinStr,
                        montoMensual = form.montoMensual, deposito = form.deposito.ifBlank { null }, moneda = form.moneda,
                    ))
                }.map { }
            } else {
                contratosRepository.create(CreateContratoRequest(
                    propiedadId = form.propiedadId, inquilinoId = form.inquilinoId,
                    fechaInicio = fechaInicioStr, fechaFin = fechaFinStr,
                    montoMensual = form.montoMensual, deposito = form.deposito.ifBlank { null }, moneda = form.moneda,
                )).map { }
            }
            _formState.update { it.copy(isSubmitting = false) }
            result.onSuccess { _successMessage.value = if (editingId != null) "Actualizado correctamente" else "Creado correctamente"; onSuccess() }
                .onFailure { e -> _formState.update { it.copy(errors = mapOf("general" to (e.message ?: "Error desconocido"))) } }
        }
    }

    fun showRenew() { _showRenewDialog.value = true; _renewForm.value = RenewFormState() }
    fun dismissRenew() { _showRenewDialog.value = false }

    fun onRenewFechaFinChange(date: LocalDate) { _renewForm.update { it.copy(fechaFin = date, errors = it.errors - "fechaFin") } }
    fun onRenewMontoChange(value: String) { _renewForm.update { it.copy(montoMensual = value, errors = it.errors - "montoMensual") } }

    fun confirmRenew() {
        val form = _renewForm.value
        val id = currentContratoId ?: return
        if (form.fechaFin == null || form.montoMensual.isBlank()) {
            _renewForm.update { it.copy(errors = buildMap {
                if (form.fechaFin == null) put("fechaFin", "La fecha de fin es requerida")
                if (form.montoMensual.isBlank()) put("montoMensual", "El monto es requerido")
            }) }
            return
        }
        viewModelScope.launch {
            contratosRepository.renew(id, RenovarContratoRequest(DateFormatter.toApi(form.fechaFin), form.montoMensual))
            _showRenewDialog.value = false
            _successMessage.value = "Contrato renovado"
        }
    }

    fun showTerminate() { _showTerminateDialog.value = true }
    fun dismissTerminate() { _showTerminateDialog.value = false }

    fun confirmTerminate() {
        val id = currentContratoId ?: return
        viewModelScope.launch {
            contratosRepository.terminate(id, TerminarContratoRequest(DateFormatter.toApi(LocalDate.now())))
            _showTerminateDialog.value = false
            _successMessage.value = "Contrato terminado"
        }
    }

    fun requestDelete(contrato: Contrato) { _deleteTarget.value = contrato }
    fun dismissDelete() { _deleteTarget.value = null }
    fun confirmDelete() {
        val target = _deleteTarget.value ?: return
        viewModelScope.launch { contratosRepository.delete(target.id); _deleteTarget.value = null; _successMessage.value = "Eliminado correctamente" }
    }
    fun clearSuccessMessage() { _successMessage.value = null }

    private suspend fun <T> Result<Unit>.flatMapSuspend(block: suspend () -> Result<T>): Result<T> =
        if (isSuccess) block() else Result.failure(exceptionOrNull()!!)
}
