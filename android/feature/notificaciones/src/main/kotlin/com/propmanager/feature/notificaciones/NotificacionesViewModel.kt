package com.propmanager.feature.notificaciones

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.data.repository.NotificacionesRepository
import com.propmanager.core.network.ConnectivityObserver
import com.propmanager.core.network.api.PagoVencidoDto
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class NotificacionesUiState(
    val isLoading: Boolean = false,
    val errorMessage: String? = null,
    val pagosVencidos: List<PagoVencidoDto> = emptyList(),
)

@HiltViewModel
class NotificacionesViewModel
@Inject
constructor(
    private val notificacionesRepository: NotificacionesRepository,
    private val networkMonitor: ConnectivityObserver,
) : ViewModel() {
    private val _uiState = MutableStateFlow(NotificacionesUiState())
    val uiState: StateFlow<NotificacionesUiState> = _uiState.asStateFlow()

    val isOnline: StateFlow<Boolean> =
        networkMonitor.isOnline.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    val badgeCount: StateFlow<Int>
        get() = _badgeCount.asStateFlow()

    private val _badgeCount = MutableStateFlow(0)

    init {
        loadPagosVencidos()
    }

    fun loadPagosVencidos() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            notificacionesRepository
                .fetchPagosVencidos()
                .onSuccess { pagos ->
                    _uiState.update { it.copy(isLoading = false, pagosVencidos = pagos) }
                    _badgeCount.value = pagos.size
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isLoading = false, errorMessage = e.message) }
                }
        }
    }
}
