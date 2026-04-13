package com.propmanager.feature.configuracion

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.data.repository.ConfiguracionRepository
import com.propmanager.core.network.NetworkMonitor
import com.propmanager.core.network.api.UpdateMonedaRequest
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class ConfiguracionUiState(
    val isLoading: Boolean = false,
    val errorMessage: String? = null,
    val tasa: String = "",
    val actualizado: String = "",
    val isSaving: Boolean = false,
    val saveSuccess: Boolean = false,
)

@HiltViewModel
class ConfiguracionViewModel
@Inject
constructor(
    private val configuracionRepository: ConfiguracionRepository,
    private val networkMonitor: NetworkMonitor,
) : ViewModel() {
    private val _uiState = MutableStateFlow(ConfiguracionUiState())
    val uiState: StateFlow<ConfiguracionUiState> = _uiState.asStateFlow()

    val isOnline: StateFlow<Boolean> =
        networkMonitor.isOnline.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    init {
        loadMoneda()
    }

    fun loadMoneda() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            configuracionRepository
                .fetchMoneda()
                .onSuccess { config ->
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            tasa = config.tasa.toString(),
                            actualizado = config.actualizado,
                        )
                    }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isLoading = false, errorMessage = e.message) }
                }
        }
    }

    fun onTasaChange(tasa: String) {
        _uiState.update { it.copy(tasa = tasa, errorMessage = null, saveSuccess = false) }
    }

    fun saveMoneda() {
        val tasaValue = _uiState.value.tasa.toDoubleOrNull()
        if (tasaValue == null) {
            _uiState.update { it.copy(errorMessage = "El valor debe ser un número válido") }
            return
        }
        viewModelScope.launch {
            _uiState.update { it.copy(isSaving = true, errorMessage = null, saveSuccess = false) }
            configuracionRepository
                .updateMoneda(UpdateMonedaRequest(tasa = tasaValue))
                .onSuccess { config ->
                    _uiState.update {
                        it.copy(
                            isSaving = false,
                            saveSuccess = true,
                            tasa = config.tasa.toString(),
                            actualizado = config.actualizado,
                        )
                    }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isSaving = false, errorMessage = e.message) }
                }
        }
    }
}
