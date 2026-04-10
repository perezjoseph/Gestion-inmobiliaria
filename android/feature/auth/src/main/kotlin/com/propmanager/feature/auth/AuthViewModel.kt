package com.propmanager.feature.auth

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.model.UserProfile
import com.propmanager.core.model.dto.LoginRequest
import com.propmanager.core.network.ApiErrorParser
import com.propmanager.core.network.TokenProvider
import com.propmanager.core.network.api.AuthApiService
import dagger.hilt.android.lifecycle.HiltViewModel
import java.io.IOException
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class LoginFormState(
    val email: String = "",
    val password: String = "",
    val isLoading: Boolean = false,
    val errorMessage: String? = null,
    val emailError: String? = null,
    val passwordError: String? = null,
)

@HiltViewModel
class AuthViewModel
@Inject
constructor(private val authApiService: AuthApiService, private val tokenProvider: TokenProvider) :
    ViewModel() {
    private val _formState = MutableStateFlow(LoginFormState())
    val formState: StateFlow<LoginFormState> = _formState.asStateFlow()

    private val _authState = MutableStateFlow<AuthState>(AuthState.Loading)
    val authState: StateFlow<AuthState> = _authState.asStateFlow()

    init {
        checkAuthState()
    }

    private fun checkAuthState() {
        val token = tokenProvider.getToken()
        val profile = tokenProvider.getUserProfile()
        _authState.value =
            if (token != null && profile != null) {
                AuthState.Authenticated(profile)
            } else {
                AuthState.Unauthenticated
            }
    }

    fun onEmailChange(email: String) {
        _formState.update { it.copy(email = email, emailError = null, errorMessage = null) }
    }

    fun onPasswordChange(password: String) {
        _formState.update {
            it.copy(password = password, passwordError = null, errorMessage = null)
        }
    }

    fun login() {
        val current = _formState.value
        val emailErr = if (current.email.isBlank()) "El correo electrónico es requerido" else null
        val passwordErr = if (current.password.isBlank()) "La contraseña es requerida" else null

        if (emailErr != null || passwordErr != null) {
            _formState.update { it.copy(emailError = emailErr, passwordError = passwordErr) }
            return
        }

        viewModelScope.launch {
            _formState.update { it.copy(isLoading = true, errorMessage = null) }
            try {
                val response =
                    authApiService.login(
                        LoginRequest(email = current.email, password = current.password)
                    )
                if (response.isSuccessful) {
                    val body = response.body()!!
                    tokenProvider.saveToken(body.token)
                    val profile =
                        UserProfile(
                            id = body.user.id,
                            nombre = body.user.nombre,
                            email = body.user.email,
                            rol = body.user.rol,
                        )
                    tokenProvider.saveUserProfile(profile)
                    _authState.value = AuthState.Authenticated(profile)
                    _formState.update { it.copy(isLoading = false) }
                } else {
                    val message = ApiErrorParser.extractMessage(response)
                    _formState.update { it.copy(isLoading = false, errorMessage = message) }
                }
            } catch (_: IOException) {
                _formState.update {
                    it.copy(isLoading = false, errorMessage = "Sin conexión a internet")
                }
            } catch (_: Exception) {
                _formState.update {
                    it.copy(
                        isLoading = false,
                        errorMessage = "Ha ocurrido un error. Intente nuevamente.",
                    )
                }
            }
        }
    }

    fun logout() {
        tokenProvider.clearAll()
        _authState.value = AuthState.Unauthenticated
        _formState.value = LoginFormState()
    }

    fun onSessionExpired() {
        tokenProvider.clearAll()
        _authState.value = AuthState.Unauthenticated
        _formState.update {
            it.copy(errorMessage = "Su sesión ha expirado. Inicie sesión nuevamente.")
        }
    }
}
