package com.propmanager.feature.perfil

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.data.repository.PerfilRepository
import com.propmanager.core.network.ConnectivityObserver
import com.propmanager.core.network.api.ChangePasswordRequest
import com.propmanager.core.network.api.UpdatePerfilRequest
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class PerfilUiState(
    val isLoading: Boolean = false,
    val errorMessage: String? = null,
    val nombre: String = "",
    val email: String = "",
    val rol: String = "",
    val isSaving: Boolean = false,
    val saveSuccess: Boolean = false,
    val passwordActual: String = "",
    val passwordNueva: String = "",
    val isChangingPassword: Boolean = false,
    val passwordChanged: Boolean = false,
    val showPasswordForm: Boolean = false,
)

@HiltViewModel
class PerfilViewModel
@Inject
constructor(
    private val perfilRepository: PerfilRepository,
    private val networkMonitor: ConnectivityObserver,
) : ViewModel() {
    private val _uiState = MutableStateFlow(PerfilUiState())
    val uiState: StateFlow<PerfilUiState> = _uiState.asStateFlow()

    val isOnline: StateFlow<Boolean> =
        networkMonitor.isOnline.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    init {
        loadPerfil()
    }

    fun loadPerfil() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            perfilRepository
                .fetchPerfil()
                .onSuccess { user ->
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            nombre = user.nombre,
                            email = user.email,
                            rol = user.rol,
                        )
                    }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isLoading = false, errorMessage = e.message) }
                }
        }
    }

    fun onNombreChange(nombre: String) {
        _uiState.update { it.copy(nombre = nombre, errorMessage = null, saveSuccess = false) }
    }

    fun savePerfil() {
        viewModelScope.launch {
            _uiState.update { it.copy(isSaving = true, errorMessage = null, saveSuccess = false) }
            perfilRepository
                .updatePerfil(UpdatePerfilRequest(nombre = _uiState.value.nombre))
                .onSuccess { user ->
                    _uiState.update {
                        it.copy(isSaving = false, saveSuccess = true, nombre = user.nombre)
                    }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isSaving = false, errorMessage = e.message) }
                }
        }
    }

    fun togglePasswordForm() {
        _uiState.update {
            it.copy(
                showPasswordForm = !it.showPasswordForm,
                passwordActual = "",
                passwordNueva = "",
                passwordChanged = false,
                errorMessage = null,
            )
        }
    }

    fun onPasswordActualChange(value: String) {
        _uiState.update { it.copy(passwordActual = value, errorMessage = null) }
    }

    fun onPasswordNuevaChange(value: String) {
        _uiState.update { it.copy(passwordNueva = value, errorMessage = null) }
    }

    fun changePassword() {
        val state = _uiState.value
        if (state.passwordActual.isBlank() || state.passwordNueva.isBlank()) {
            _uiState.update { it.copy(errorMessage = "Ambos campos de contraseña son requeridos") }
            return
        }
        viewModelScope.launch {
            _uiState.update { it.copy(isChangingPassword = true, errorMessage = null) }
            perfilRepository
                .changePassword(
                    ChangePasswordRequest(
                        passwordActual = state.passwordActual,
                        passwordNuevo = state.passwordNueva,
                    )
                )
                .onSuccess {
                    _uiState.update {
                        it.copy(
                            isChangingPassword = false,
                            passwordChanged = true,
                            passwordActual = "",
                            passwordNueva = "",
                            showPasswordForm = false,
                        )
                    }
                }
                .onFailure { e ->
                    _uiState.update {
                        it.copy(isChangingPassword = false, errorMessage = e.message)
                    }
                }
        }
    }
}
