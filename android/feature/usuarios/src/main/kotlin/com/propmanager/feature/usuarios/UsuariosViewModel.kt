package com.propmanager.feature.usuarios

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.model.dto.ChangeRoleRequest
import com.propmanager.core.model.dto.UserDto
import com.propmanager.core.network.api.UsuariosApiService
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.toImmutableList
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

sealed interface UsuariosUiState {
    data object Loading : UsuariosUiState

    data class Success(
        val users: ImmutableList<UserDto>,
        val page: Int,
        val totalPages: Int,
    ) : UsuariosUiState

    data class Error(val message: String) : UsuariosUiState
}

@HiltViewModel
class UsuariosViewModel
@Inject
constructor(
    private val apiService: UsuariosApiService,
) : ViewModel() {

    private val _uiState = MutableStateFlow<UsuariosUiState>(UsuariosUiState.Loading)
    val uiState: StateFlow<UsuariosUiState> = _uiState.asStateFlow()

    private val _actionError = MutableStateFlow<String?>(null)
    val actionError: StateFlow<String?> = _actionError.asStateFlow()

    companion object {
        private const val PER_PAGE = 20
    }

    init {
        loadUsuarios(page = 1)
    }

    fun loadUsuarios(page: Int) {
        viewModelScope.launch {
            _uiState.value = UsuariosUiState.Loading
            try {
                val response = apiService.getUsuarios(page = page, perPage = PER_PAGE)
                val body = response.body()
                if (response.isSuccessful && body != null) {
                    val totalPages =
                        ((body.total + PER_PAGE - 1) / PER_PAGE).toInt()
                    _uiState.value =
                        UsuariosUiState.Success(
                            users = body.data.toImmutableList(),
                            page = page,
                            totalPages = totalPages,
                        )
                } else {
                    _uiState.value =
                        UsuariosUiState.Error("Error al cargar usuarios: ${response.code()}")
                }
            } catch (e: Exception) {
                _uiState.value =
                    UsuariosUiState.Error("Error de conexión al cargar usuarios")
            }
        }
    }

    fun changeRole(userId: String, newRole: String) {
        viewModelScope.launch {
            _actionError.value = null
            try {
                val response = apiService.changeRole(userId, ChangeRoleRequest(rol = newRole))
                if (response.isSuccessful && response.body() != null) {
                    val updatedUser = response.body()!!
                    updateUserInList(updatedUser)
                } else {
                    _actionError.value = "Error al cambiar rol: ${response.code()}"
                }
            } catch (e: Exception) {
                _actionError.value = "Error de conexión al cambiar rol"
            }
        }
    }

    fun toggleActivo(userId: String) {
        viewModelScope.launch {
            _actionError.value = null
            try {
                val response = apiService.toggleActivo(userId)
                if (response.isSuccessful && response.body() != null) {
                    val updatedUser = response.body()!!
                    updateUserInList(updatedUser)
                } else {
                    _actionError.value = "Error al cambiar estado: ${response.code()}"
                }
            } catch (e: Exception) {
                _actionError.value = "Error de conexión al cambiar estado"
            }
        }
    }

    fun clearActionError() {
        _actionError.value = null
    }

    private fun updateUserInList(updatedUser: UserDto) {
        val current = _uiState.value
        if (current is UsuariosUiState.Success) {
            _uiState.update {
                current.copy(
                    users =
                        current.users
                            .map { if (it.id == updatedUser.id) updatedUser else it }
                            .toImmutableList(),
                )
            }
        }
    }
}
