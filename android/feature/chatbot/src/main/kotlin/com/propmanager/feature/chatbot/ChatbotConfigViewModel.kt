package com.propmanager.feature.chatbot

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.model.dto.ChatbotConfigResponse
import com.propmanager.core.model.dto.ChatbotConfigUpdateRequest
import com.propmanager.core.model.dto.ConnectionStatusResponse
import com.propmanager.core.model.dto.ReceiptConfirmRequest
import com.propmanager.core.model.dto.ReceiptExtractionResponse
import com.propmanager.core.model.dto.ReceiptRejectRequest
import com.propmanager.core.model.dto.TestChatHistoryEntry
import com.propmanager.core.model.dto.TestChatRequest
import com.propmanager.core.network.api.ChatbotApiService
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.collections.immutable.ImmutableList
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.toImmutableList
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

sealed interface ChatbotConfigUiState {
    data object Loading : ChatbotConfigUiState

    data class Success(
        val config: ChatbotConfigResponse,
        val connectionStatus: ConnectionStatusResponse,
        val currentStep: Int,
    ) : ChatbotConfigUiState

    data class Error(val message: String) : ChatbotConfigUiState
}

data class ChatMessage(
    val role: String,
    val content: String,
    val toolsInvoked: List<String> = emptyList(),
)

@HiltViewModel
class ChatbotConfigViewModel
@Inject
constructor(
    private val apiService: ChatbotApiService,
) : ViewModel() {

    private val _uiState = MutableStateFlow<ChatbotConfigUiState>(ChatbotConfigUiState.Loading)
    val uiState: StateFlow<ChatbotConfigUiState> = _uiState.asStateFlow()

    private val _pendingReceipts = MutableStateFlow<ImmutableList<ReceiptExtractionResponse>>(persistentListOf())
    val pendingReceipts: StateFlow<ImmutableList<ReceiptExtractionResponse>> = _pendingReceipts.asStateFlow()

    private val _chatMessages = MutableStateFlow<ImmutableList<ChatMessage>>(persistentListOf())
    val chatMessages: StateFlow<ImmutableList<ChatMessage>> = _chatMessages.asStateFlow()

    private val _actionError = MutableStateFlow<String?>(null)
    val actionError: StateFlow<String?> = _actionError.asStateFlow()

    private val _isActionLoading = MutableStateFlow(false)
    val isActionLoading: StateFlow<Boolean> = _isActionLoading.asStateFlow()

    companion object {
        const val TOTAL_STEPS = 7
    }

    init {
        loadConfig()
    }

    fun loadConfig() {
        viewModelScope.launch {
            _uiState.value = ChatbotConfigUiState.Loading
            try {
                val configResponse = apiService.getConfig()
                val statusResponse = apiService.getStatus()

                val config = configResponse.body()
                val status = statusResponse.body()

                if (configResponse.isSuccessful && config != null &&
                    statusResponse.isSuccessful && status != null
                ) {
                    _uiState.value = ChatbotConfigUiState.Success(
                        config = config,
                        connectionStatus = status,
                        currentStep = 0,
                    )
                } else {
                    _uiState.value = ChatbotConfigUiState.Error(
                        "Error al cargar configuración del chatbot: ${configResponse.code()}",
                    )
                }
            } catch (e: Exception) {
                _uiState.value = ChatbotConfigUiState.Error(
                    "Error de conexión al cargar configuración del chatbot",
                )
            }
        }
    }

    fun nextStep() {
        val current = _uiState.value
        if (current is ChatbotConfigUiState.Success && current.currentStep < TOTAL_STEPS - 1) {
            _uiState.value = current.copy(currentStep = current.currentStep + 1)
        }
    }

    fun previousStep() {
        val current = _uiState.value
        if (current is ChatbotConfigUiState.Success && current.currentStep > 0) {
            _uiState.value = current.copy(currentStep = current.currentStep - 1)
        }
    }

    fun updateConfig(request: ChatbotConfigUpdateRequest) {
        viewModelScope.launch {
            _actionError.value = null
            _isActionLoading.value = true
            try {
                val response = apiService.updateConfig(request)
                val body = response.body()
                if (response.isSuccessful && body != null) {
                    val current = _uiState.value
                    if (current is ChatbotConfigUiState.Success) {
                        _uiState.value = current.copy(config = body)
                    }
                } else {
                    _actionError.value = "Error al actualizar configuración: ${response.code()}"
                }
            } catch (e: Exception) {
                _actionError.value = "Error de conexión al actualizar configuración"
            } finally {
                _isActionLoading.value = false
            }
        }
    }

    fun connect() {
        viewModelScope.launch {
            _actionError.value = null
            _isActionLoading.value = true
            try {
                val response = apiService.connect()
                val body = response.body()
                if (response.isSuccessful && body != null) {
                    val current = _uiState.value
                    if (current is ChatbotConfigUiState.Success) {
                        _uiState.value = current.copy(connectionStatus = body)
                    }
                } else {
                    _actionError.value = "Error al conectar: ${response.code()}"
                }
            } catch (e: Exception) {
                _actionError.value = "Error de conexión al intentar conectar"
            } finally {
                _isActionLoading.value = false
            }
        }
    }

    fun disconnect() {
        viewModelScope.launch {
            _actionError.value = null
            _isActionLoading.value = true
            try {
                val response = apiService.disconnect()
                val body = response.body()
                if (response.isSuccessful && body != null) {
                    val current = _uiState.value
                    if (current is ChatbotConfigUiState.Success) {
                        _uiState.value = current.copy(connectionStatus = body)
                    }
                } else {
                    _actionError.value = "Error al desconectar: ${response.code()}"
                }
            } catch (e: Exception) {
                _actionError.value = "Error de conexión al intentar desconectar"
            } finally {
                _isActionLoading.value = false
            }
        }
    }

    fun testChat(message: String) {
        viewModelScope.launch {
            _actionError.value = null
            _isActionLoading.value = true

            val userMessage = ChatMessage(role = "user", content = message)
            _chatMessages.value = (_chatMessages.value + userMessage).toImmutableList()

            try {
                val history = _chatMessages.value
                    .filter { it.role == "user" || it.role == "assistant" }
                    .dropLast(1)
                    .map { TestChatHistoryEntry(role = it.role, content = it.content) }

                val request = TestChatRequest(
                    message = message,
                    history = history,
                )
                val response = apiService.testChat(request)
                val body = response.body()
                if (response.isSuccessful && body != null) {
                    val assistantMessage = ChatMessage(
                        role = "assistant",
                        content = body.reply,
                        toolsInvoked = body.toolsInvoked,
                    )
                    _chatMessages.value = (_chatMessages.value + assistantMessage).toImmutableList()
                } else {
                    _actionError.value = "Error al enviar mensaje de prueba: ${response.code()}"
                }
            } catch (e: Exception) {
                _actionError.value = "Error de conexión al enviar mensaje de prueba"
            } finally {
                _isActionLoading.value = false
            }
        }
    }

    fun clearChatHistory() {
        _chatMessages.value = persistentListOf()
    }

    fun loadPendingReceipts() {
        viewModelScope.launch {
            _actionError.value = null
            try {
                val response = apiService.getPendingReceipts()
                val body = response.body()
                if (response.isSuccessful && body != null) {
                    _pendingReceipts.value = body.toImmutableList()
                } else {
                    _actionError.value = "Error al cargar recibos pendientes: ${response.code()}"
                }
            } catch (e: Exception) {
                _actionError.value = "Error de conexión al cargar recibos pendientes"
            }
        }
    }

    fun confirmReceipt(receiptId: String, contratoId: String? = null) {
        viewModelScope.launch {
            _actionError.value = null
            _isActionLoading.value = true
            try {
                val request = ReceiptConfirmRequest(contratoId = contratoId)
                val response = apiService.confirmReceipt(receiptId, request)
                if (response.isSuccessful) {
                    _pendingReceipts.value = _pendingReceipts.value
                        .filter { it.id != receiptId }
                        .toImmutableList()
                } else {
                    _actionError.value = "Error al confirmar recibo: ${response.code()}"
                }
            } catch (e: Exception) {
                _actionError.value = "Error de conexión al confirmar recibo"
            } finally {
                _isActionLoading.value = false
            }
        }
    }

    fun rejectReceipt(receiptId: String, reason: String? = null) {
        viewModelScope.launch {
            _actionError.value = null
            _isActionLoading.value = true
            try {
                val request = ReceiptRejectRequest(rejectionReason = reason)
                val response = apiService.rejectReceipt(receiptId, request)
                if (response.isSuccessful) {
                    _pendingReceipts.value = _pendingReceipts.value
                        .filter { it.id != receiptId }
                        .toImmutableList()
                } else {
                    _actionError.value = "Error al rechazar recibo: ${response.code()}"
                }
            } catch (e: Exception) {
                _actionError.value = "Error de conexión al rechazar recibo"
            } finally {
                _isActionLoading.value = false
            }
        }
    }

    fun clearActionError() {
        _actionError.value = null
    }
}
