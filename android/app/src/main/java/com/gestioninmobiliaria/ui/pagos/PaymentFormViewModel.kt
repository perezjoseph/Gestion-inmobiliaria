package com.gestioninmobiliaria.ui.pagos

import androidx.lifecycle.SavedStateHandle
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.gestioninmobiliaria.data.repository.PagoRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

@HiltViewModel
class PaymentFormViewModel @Inject constructor(
    private val savedState: SavedStateHandle,
    private val repository: PagoRepository,
) : ViewModel() {

    val contratoId = savedState.getStateFlow(KEY_CONTRATO_ID, "")
    val monto = savedState.getStateFlow(KEY_MONTO, "")
    val moneda = savedState.getStateFlow(KEY_MONEDA, "DOP")
    val metodoPago = savedState.getStateFlow(KEY_METODO_PAGO, "")
    val fechaPago = savedState.getStateFlow(KEY_FECHA_PAGO, "")

    private val _isSaving = MutableStateFlow(false)
    val isSaving: StateFlow<Boolean> = _isSaving

    private val _saveError = MutableStateFlow<String?>(null)
    val saveError: StateFlow<String?> = _saveError

    private val _saved = MutableStateFlow(false)
    val saved: StateFlow<Boolean> = _saved

    fun updateContratoId(value: String) { savedState[KEY_CONTRATO_ID] = value }
    fun updateMonto(value: String) { savedState[KEY_MONTO] = value }
    fun updateMoneda(value: String) { savedState[KEY_MONEDA] = value }
    fun updateMetodoPago(value: String) { savedState[KEY_METODO_PAGO] = value }
    fun updateFechaPago(value: String) { savedState[KEY_FECHA_PAGO] = value }

    fun submit() {
        viewModelScope.launch {
            _isSaving.value = true
            _saveError.value = null
            try {
                val parsedMonto = monto.value.toDoubleOrNull()
                    ?: throw IllegalArgumentException("Monto inválido")
                val parsedContratoId = contratoId.value.toIntOrNull()
                    ?: throw IllegalArgumentException("Contrato inválido")

                repository.createPago(
                    contratoId = parsedContratoId,
                    monto = parsedMonto,
                    moneda = moneda.value,
                    metodoPago = metodoPago.value,
                    fechaPago = fechaPago.value,
                )
                clearDraft()
                _saved.value = true
            } catch (e: Exception) {
                _saveError.value = e.message ?: "Error al guardar pago"
            } finally {
                _isSaving.value = false
            }
        }
    }

    fun clearDraft() {
        savedState[KEY_CONTRATO_ID] = ""
        savedState[KEY_MONTO] = ""
        savedState[KEY_MONEDA] = "DOP"
        savedState[KEY_METODO_PAGO] = ""
        savedState[KEY_FECHA_PAGO] = ""
    }

    companion object {
        private const val KEY_CONTRATO_ID = "draft_contrato_id"
        private const val KEY_MONTO = "draft_monto"
        private const val KEY_MONEDA = "draft_moneda"
        private const val KEY_METODO_PAGO = "draft_metodo_pago"
        private const val KEY_FECHA_PAGO = "draft_fecha_pago"
    }
}
