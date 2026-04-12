package com.propmanager.feature.importacion

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.data.repository.ImportacionRepository
import com.propmanager.core.network.NetworkMonitor
import com.propmanager.core.network.api.ImportErrorDto
import com.propmanager.core.network.api.ImportResultDto
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import okhttp3.MultipartBody
import javax.inject.Inject

enum class ImportType { PROPIEDADES, INQUILINOS, GASTOS }

data class ImportacionUiState(
    val isLoading: Boolean = false,
    val errorMessage: String? = null,
    val selectedType: ImportType = ImportType.PROPIEDADES,
    val result: ImportResultDto? = null,
)

@HiltViewModel
class ImportacionViewModel @Inject constructor(
    private val importacionRepository: ImportacionRepository,
    private val networkMonitor: NetworkMonitor,
) : ViewModel() {

    private val _uiState = MutableStateFlow(ImportacionUiState())
    val uiState: StateFlow<ImportacionUiState> = _uiState.asStateFlow()

    val isOnline: StateFlow<Boolean> = networkMonitor.isOnline
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    fun selectType(type: ImportType) {
        _uiState.update { it.copy(selectedType = type, result = null, errorMessage = null) }
    }

    fun importFile(file: MultipartBody.Part) {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null, result = null) }
            val result = when (_uiState.value.selectedType) {
                ImportType.PROPIEDADES -> importacionRepository.importPropiedades(file)
                ImportType.INQUILINOS -> importacionRepository.importInquilinos(file)
                ImportType.GASTOS -> importacionRepository.importGastos(file)
            }
            result
                .onSuccess { dto -> _uiState.update { it.copy(isLoading = false, result = dto) } }
                .onFailure { e -> _uiState.update { it.copy(isLoading = false, errorMessage = e.message) } }
        }
    }

    fun clearResult() {
        _uiState.update { it.copy(result = null, errorMessage = null) }
    }
}
