package com.gestioninmobiliaria.ui.pagos

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.gestioninmobiliaria.data.model.EstadoPago
import com.gestioninmobiliaria.data.model.Pago
import com.gestioninmobiliaria.data.repository.PagoRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

data class PaymentListUiState(
    val grouped: Map<EstadoPago, List<Pago>> = emptyMap(),
    val isLoading: Boolean = false,
    val error: String? = null,
)

@HiltViewModel
class PaymentListViewModel @Inject constructor(
    private val repository: PagoRepository,
) : ViewModel() {

    private val _uiState = MutableStateFlow(PaymentListUiState(isLoading = true))
    val uiState: StateFlow<PaymentListUiState> = _uiState

    init { load() }

    fun load() {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isLoading = true, error = null)
            try {
                val pagos = repository.getPagos()
                val grouped = pagos.groupBy { it.estado }
                    .toSortedMap(compareBy { it.ordinal })
                _uiState.value = PaymentListUiState(grouped = grouped)
            } catch (e: Exception) {
                _uiState.value = PaymentListUiState(error = e.message ?: "Error desconocido")
            }
        }
    }
}
