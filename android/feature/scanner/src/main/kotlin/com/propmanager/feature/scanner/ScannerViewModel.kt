package com.propmanager.feature.scanner

import androidx.lifecycle.ViewModel
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class ScannerUiState(
    val isProcessing: Boolean = false,
    val errorMessage: String? = null,
    val cedulaResult: CedulaOcrResult? = null,
    val receiptResult: ReceiptOcrResult? = null,
)

@HiltViewModel
class ScannerViewModel
@Inject
constructor(
    @Suppress("UnusedPrivateProperty") private val cedulaOcrExtractor: CedulaOcrExtractor,
    @Suppress("UnusedPrivateProperty") private val receiptOcrExtractor: ReceiptOcrExtractor,
) : ViewModel() {
    private val _uiState = MutableStateFlow(ScannerUiState())
    val uiState: StateFlow<ScannerUiState> = _uiState.asStateFlow()

    @Suppress("UnusedParameter")
    fun onCaptureRequested(mode: ScannerMode) {
        _uiState.update {
            it.copy(
                isProcessing = true,
                errorMessage = null,
                cedulaResult = null,
                receiptResult = null,
            )
        }
        // Camera capture + ML Kit processing will be triggered by the camera integration.
        // For now, the processing state is set; actual image capture requires CameraX integration
        // which will call processCedulaImage or processReceiptImage with the captured InputImage.
        _uiState.update {
            it.copy(isProcessing = false, errorMessage = "Cámara no disponible en esta versión")
        }
    }

    fun reset() {
        _uiState.value = ScannerUiState()
    }
}
