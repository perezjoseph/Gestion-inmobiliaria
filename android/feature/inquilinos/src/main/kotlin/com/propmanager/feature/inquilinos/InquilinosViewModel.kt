package com.propmanager.feature.inquilinos

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.common.InquilinoValidator
import com.propmanager.core.data.repository.InquilinosRepository
import com.propmanager.core.model.Inquilino
import com.propmanager.core.model.ValidationResult
import com.propmanager.core.model.dto.CreateInquilinoRequest
import com.propmanager.core.model.dto.UpdateInquilinoRequest
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

data class InquilinoFormState(
    val nombre: String = "",
    val apellido: String = "",
    val cedula: String = "",
    val email: String = "",
    val telefono: String = "",
    val contactoEmergencia: String = "",
    val notas: String = "",
    val errors: Map<String, String> = emptyMap(),
    val isSubmitting: Boolean = false,
)

sealed interface InquilinosUiState {
    data object Loading : InquilinosUiState

    data class Success(val inquilinos: List<Inquilino>) : InquilinosUiState
}

@OptIn(ExperimentalCoroutinesApi::class)
@HiltViewModel
class InquilinosViewModel
@Inject
constructor(
    private val repository: InquilinosRepository,
    private val networkMonitor: ConnectivityObserver,
) : ViewModel() {
    private val _searchQuery = MutableStateFlow("")
    val searchQuery: StateFlow<String> = _searchQuery.asStateFlow()

    val inquilinos: StateFlow<InquilinosUiState> =
        MutableStateFlow<InquilinosUiState>(InquilinosUiState.Loading).also { state ->
            viewModelScope.launch {
                _searchQuery
                    .flatMapLatest { query ->
                        if (query.isBlank()) repository.observeAll() else repository.search(query)
                    }
                    .collect { list -> state.value = InquilinosUiState.Success(list) }
            }
        }

    private val _formState = MutableStateFlow(InquilinoFormState())
    val formState: StateFlow<InquilinoFormState> = _formState.asStateFlow()

    private val _successMessage = MutableStateFlow<String?>(null)
    val successMessage: StateFlow<String?> = _successMessage.asStateFlow()

    private val _deleteTarget = MutableStateFlow<Inquilino?>(null)
    val deleteTarget: StateFlow<Inquilino?> = _deleteTarget.asStateFlow()

    val isOnline: StateFlow<Boolean> =
        networkMonitor.isOnline.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    private var editingId: String? = null

    fun onSearchChange(query: String) {
        _searchQuery.value = query
    }

    fun initCreateForm() {
        editingId = null
        _formState.value = InquilinoFormState()
    }

    fun initEditForm(inquilino: Inquilino) {
        editingId = inquilino.id
        _formState.value =
            InquilinoFormState(
                nombre = inquilino.nombre,
                apellido = inquilino.apellido,
                cedula = inquilino.cedula,
                email = inquilino.email ?: "",
                telefono = inquilino.telefono ?: "",
                contactoEmergencia = inquilino.contactoEmergencia ?: "",
                notas = inquilino.notas ?: "",
            )
    }

    fun prefillFromOcr(nombre: String?, apellido: String?, cedula: String?) {
        _formState.update {
            it.copy(
                nombre = nombre ?: it.nombre,
                apellido = apellido ?: it.apellido,
                cedula = cedula ?: it.cedula,
            )
        }
    }

    fun onFieldChange(field: String, value: String) {
        _formState.update { state ->
            val newErrors = state.errors - field
            when (field) {
                "nombre" -> state.copy(nombre = value, errors = newErrors)
                "apellido" -> state.copy(apellido = value, errors = newErrors)
                "cedula" -> state.copy(cedula = value, errors = newErrors)
                "email" -> state.copy(email = value, errors = newErrors)
                "telefono" -> state.copy(telefono = value, errors = newErrors)
                "contactoEmergencia" -> state.copy(contactoEmergencia = value, errors = newErrors)
                "notas" -> state.copy(notas = value, errors = newErrors)
                else -> state
            }
        }
    }

    fun save(onSuccess: () -> Unit) {
        val form = _formState.value
        val validation = InquilinoValidator.validateCreate(form.nombre, form.apellido, form.cedula)
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
                        UpdateInquilinoRequest(
                            nombre = form.nombre,
                            apellido = form.apellido,
                            cedula = form.cedula,
                            email = form.email.ifBlank { null },
                            telefono = form.telefono.ifBlank { null },
                            contactoEmergencia = form.contactoEmergencia.ifBlank { null },
                            notas = form.notas.ifBlank { null },
                        ),
                    )
                } else {
                    repository
                        .create(
                            CreateInquilinoRequest(
                                nombre = form.nombre,
                                apellido = form.apellido,
                                cedula = form.cedula,
                                email = form.email.ifBlank { null },
                                telefono = form.telefono.ifBlank { null },
                                contactoEmergencia = form.contactoEmergencia.ifBlank { null },
                                notas = form.notas.ifBlank { null },
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

    fun requestDelete(inquilino: Inquilino) {
        _deleteTarget.value = inquilino
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
