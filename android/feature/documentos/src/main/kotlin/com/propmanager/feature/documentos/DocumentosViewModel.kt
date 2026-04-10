package com.propmanager.feature.documentos

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.data.repository.DocumentosRepository
import com.propmanager.core.network.ConnectivityObserver
import com.propmanager.core.network.api.DocumentoDto
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import okhttp3.MultipartBody

data class DocumentosUiState(
    val isLoading: Boolean = false,
    val errorMessage: String? = null,
    val documents: List<DocumentoDto> = emptyList(),
    val isUploading: Boolean = false,
    val uploadSuccess: Boolean = false,
)

@HiltViewModel
class DocumentosViewModel
@Inject
constructor(
    private val documentosRepository: DocumentosRepository,
    private val networkMonitor: ConnectivityObserver,
) : ViewModel() {
    private val _uiState = MutableStateFlow(DocumentosUiState())
    val uiState: StateFlow<DocumentosUiState> = _uiState.asStateFlow()

    val isOnline: StateFlow<Boolean> =
        networkMonitor.isOnline.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    private var currentEntityType: String = ""
    private var currentEntityId: String = ""

    fun loadDocuments(entityType: String, entityId: String) {
        currentEntityType = entityType
        currentEntityId = entityId
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            documentosRepository
                .fetchDocuments(entityType, entityId)
                .onSuccess { docs ->
                    _uiState.update { it.copy(isLoading = false, documents = docs) }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isLoading = false, errorMessage = e.message) }
                }
        }
    }

    fun uploadDocument(file: MultipartBody.Part) {
        viewModelScope.launch {
            _uiState.update {
                it.copy(isUploading = true, uploadSuccess = false, errorMessage = null)
            }
            documentosRepository
                .uploadDocument(currentEntityType, currentEntityId, file)
                .onSuccess {
                    _uiState.update { it.copy(isUploading = false, uploadSuccess = true) }
                    loadDocuments(currentEntityType, currentEntityId)
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isUploading = false, errorMessage = e.message) }
                }
        }
    }

    fun clearUploadSuccess() {
        _uiState.update { it.copy(uploadSuccess = false) }
    }
}
