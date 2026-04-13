package com.propmanager.feature.reportes

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.data.repository.ReportesRepository
import com.propmanager.core.model.dto.HistorialPagoReporte
import com.propmanager.core.model.dto.IngresoReporteSummary
import com.propmanager.core.model.dto.OcupacionTendencia
import com.propmanager.core.model.dto.RentabilidadReporteSummary
import com.propmanager.core.network.NetworkMonitor
import dagger.hilt.android.lifecycle.HiltViewModel
import dagger.hilt.android.qualifiers.ApplicationContext
import java.io.File
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import okhttp3.ResponseBody

enum class ReportType {
    INGRESOS,
    RENTABILIDAD,
    HISTORIAL_PAGOS,
    OCUPACION,
}

data class ReportesUiState(
    val isLoading: Boolean = false,
    val errorMessage: String? = null,
    val selectedReport: ReportType = ReportType.INGRESOS,
    val ingresos: IngresoReporteSummary? = null,
    val rentabilidad: RentabilidadReporteSummary? = null,
    val historialPagos: List<HistorialPagoReporte> = emptyList(),
    val ocupacion: List<OcupacionTendencia> = emptyList(),
    val isExporting: Boolean = false,
    val exportSuccess: String? = null,
)

@HiltViewModel
class ReportesViewModel
@Inject
constructor(
    private val reportesRepository: ReportesRepository,
    private val networkMonitor: NetworkMonitor,
    @ApplicationContext private val context: Context,
) : ViewModel() {
    private val _uiState = MutableStateFlow(ReportesUiState())
    val uiState: StateFlow<ReportesUiState> = _uiState.asStateFlow()

    val isOnline: StateFlow<Boolean> =
        networkMonitor.isOnline.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    fun selectReport(type: ReportType) {
        _uiState.update { it.copy(selectedReport = type, errorMessage = null) }
        loadReport(type)
    }

    fun loadReport(type: ReportType = _uiState.value.selectedReport) {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            val result =
                when (type) {
                    ReportType.INGRESOS ->
                        reportesRepository.fetchIngresos().map {
                            _uiState.update { s -> s.copy(ingresos = it) }
                        }
                    ReportType.RENTABILIDAD ->
                        reportesRepository.fetchRentabilidad().map {
                            _uiState.update { s -> s.copy(rentabilidad = it) }
                        }
                    ReportType.HISTORIAL_PAGOS ->
                        reportesRepository.fetchHistorialPagos().map {
                            _uiState.update { s -> s.copy(historialPagos = it) }
                        }
                    ReportType.OCUPACION ->
                        reportesRepository.fetchOcupacionTendencia().map {
                            _uiState.update { s -> s.copy(ocupacion = it) }
                        }
                }
            _uiState.update {
                it.copy(isLoading = false, errorMessage = result.exceptionOrNull()?.message)
            }
        }
    }

    fun exportPdf() {
        export("pdf")
    }

    fun exportXlsx() {
        export("xlsx")
    }

    private fun export(format: String) {
        viewModelScope.launch {
            _uiState.update { it.copy(isExporting = true, exportSuccess = null) }
            val result: Result<ResponseBody> =
                when (_uiState.value.selectedReport) {
                    ReportType.INGRESOS ->
                        if (format == "pdf") {
                            reportesRepository.downloadIngresosPdf()
                        } else {
                            reportesRepository.downloadIngresosXlsx()
                        }
                    ReportType.RENTABILIDAD ->
                        if (format == "pdf") {
                            reportesRepository.downloadRentabilidadPdf()
                        } else {
                            reportesRepository.downloadRentabilidadXlsx()
                        }
                    else -> {
                        _uiState.update { it.copy(isExporting = false) }
                        return@launch
                    }
                }
            result
                .onSuccess { body ->
                    val ext = if (format == "pdf") "pdf" else "xlsx"
                    val file = File(context.cacheDir, "reporte_${System.currentTimeMillis()}.$ext")
                    file.outputStream().use { body.byteStream().copyTo(it) }
                    _uiState.update {
                        it.copy(isExporting = false, exportSuccess = file.absolutePath)
                    }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isExporting = false, errorMessage = e.message) }
                }
        }
    }

    fun clearExportSuccess() {
        _uiState.update { it.copy(exportSuccess = null) }
    }
}
