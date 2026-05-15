package com.propmanager.feature.plantillas

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.model.dto.ActualizarPlantillaRequest
import com.propmanager.core.model.dto.CrearPlantillaRequest
import com.propmanager.core.model.dto.PlantillaResponse
import com.propmanager.core.network.api.PlantillasApiService
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.toImmutableList
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed interface PlantillasUiState {
    data object Loading : PlantillasUiState

    data class Success(val plantillas: ImmutableList<PlantillaResponse>) : PlantillasUiState

    data class Error(val message: String) : PlantillasUiState
}

sealed interface PlantillaFormUiState {
    data object Idle : PlantillaFormUiState

    data object Saving : PlantillaFormUiState

    data class Success(val plantilla: PlantillaResponse) : PlantillaFormUiState

    data class Error(val message: String) : PlantillaFormUiState
}

@HiltViewModel
class PlantillasViewModel
@Inject
constructor(
    private val apiService: PlantillasApiService,
) : ViewModel() {

    private val _uiState = MutableStateFlow<PlantillasUiState>(PlantillasUiState.Loading)
    val uiState: StateFlow<PlantillasUiState> = _uiState.asStateFlow()

    private val _formState = MutableStateFlow<PlantillaFormUiState>(PlantillaFormUiState.Idle)
    val formState: StateFlow<PlantillaFormUiState> = _formState.asStateFlow()

    private val _actionError = MutableStateFlow<String?>(null)
    val actionError: StateFlow<String?> = _actionError.asStateFlow()

    init {
        loadPlantillas()
    }

    fun loadPlantillas() {
        viewModelScope.launch {
            _uiState.value = PlantillasUiState.Loading
            try {
                val response = apiService.getPlantillas()
                val body = response.body()
                if (response.isSuccessful && body != null) {
                    _uiState.value = PlantillasUiState.Success(
                        plantillas = body.toImmutableList(),
                    )
                } else {
                    _uiState.value = PlantillasUiState.Error(
                        "Error al cargar plantillas: ${response.code()}",
                    )
                }
            } catch (e: Exception) {
                _uiState.value = PlantillasUiState.Error(
                    "Error de conexión al cargar plantillas",
                )
            }
        }
    }

    fun createPlantilla(request: CrearPlantillaRequest) {
        viewModelScope.launch {
            _formState.value = PlantillaFormUiState.Saving
            try {
                val response = apiService.createPlantilla(request)
                val body = response.body()
                if (response.isSuccessful && body != null) {
                    _formState.value = PlantillaFormUiState.Success(body)
                    loadPlantillas()
                } else {
                    _formState.value = PlantillaFormUiState.Error(
                        "Error al crear plantilla: ${response.code()}",
                    )
                }
            } catch (e: Exception) {
                _formState.value = PlantillaFormUiState.Error(
                    "Error de conexión al crear plantilla",
                )
            }
        }
    }

    fun updatePlantilla(id: String, request: ActualizarPlantillaRequest) {
        viewModelScope.launch {
            _formState.value = PlantillaFormUiState.Saving
            try {
                val response = apiService.updatePlantilla(id, request)
                val body = response.body()
                if (response.isSuccessful && body != null) {
                    _formState.value = PlantillaFormUiState.Success(body)
                    loadPlantillas()
                } else {
                    _formState.value = PlantillaFormUiState.Error(
                        "Error al actualizar plantilla: ${response.code()}",
                    )
                }
            } catch (e: Exception) {
                _formState.value = PlantillaFormUiState.Error(
                    "Error de conexión al actualizar plantilla",
                )
            }
        }
    }

    fun deletePlantilla(id: String) {
        viewModelScope.launch {
            _actionError.value = null
            try {
                val response = apiService.deletePlantilla(id)
                if (response.isSuccessful) {
                    loadPlantillas()
                } else {
                    _actionError.value = "Error al eliminar plantilla: ${response.code()}"
                }
            } catch (e: Exception) {
                _actionError.value = "Error de conexión al eliminar plantilla"
            }
        }
    }

    fun resetFormState() {
        _formState.value = PlantillaFormUiState.Idle
    }

    fun clearActionError() {
        _actionError.value = null
    }
}
