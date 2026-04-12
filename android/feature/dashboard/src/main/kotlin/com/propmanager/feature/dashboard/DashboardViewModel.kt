package com.propmanager.feature.dashboard

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.data.repository.DashboardRepository
import com.propmanager.core.model.dto.ContratoCalendario
import com.propmanager.core.model.dto.DashboardStats
import com.propmanager.core.model.dto.GastosComparacion
import com.propmanager.core.model.dto.IngresosComparacion
import com.propmanager.core.model.dto.OcupacionTendencia
import com.propmanager.core.model.dto.PagoProximo
import com.propmanager.core.network.NetworkMonitor
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import javax.inject.Inject

data class DashboardUiState(
    val isLoading: Boolean = true,
    val errorMessage: String? = null,
    val stats: DashboardStats? = null,
    val pagosProximos: List<PagoProximo> = emptyList(),
    val contratosCalendario: List<ContratoCalendario> = emptyList(),
    val ocupacionTendencia: List<OcupacionTendencia> = emptyList(),
    val ingresosComparacion: IngresosComparacion? = null,
    val gastosComparacion: GastosComparacion? = null,
    val isFromCache: Boolean = false,
    val lastUpdated: String? = null,
)

@HiltViewModel
class DashboardViewModel @Inject constructor(
    private val dashboardRepository: DashboardRepository,
    private val networkMonitor: NetworkMonitor,
) : ViewModel() {

    private val _uiState = MutableStateFlow(DashboardUiState())
    val uiState: StateFlow<DashboardUiState> = _uiState.asStateFlow()

    val isOnline: StateFlow<Boolean> = networkMonitor.isOnline
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    init {
        loadDashboard()
    }

    fun loadDashboard() {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }

            if (networkMonitor.isOnline.value) {
                fetchFromNetwork()
            } else {
                loadFromCache()
            }
        }
    }

    private suspend fun fetchFromNetwork() {
        val statsResult = dashboardRepository.fetchStats()
        val pagosResult = dashboardRepository.fetchPagosProximos()
        val contratosResult = dashboardRepository.fetchContratosCalendario()
        val ocupacionResult = dashboardRepository.fetchOcupacionTendencia()
        val ingresosResult = dashboardRepository.fetchIngresosComparacion()
        val gastosResult = dashboardRepository.fetchGastosComparacion()

        if (statsResult.isFailure &&
            pagosResult.isFailure &&
            contratosResult.isFailure
        ) {
            loadFromCache(fallbackError = statsResult.exceptionOrNull()?.message)
            return
        }

        _uiState.update {
            it.copy(
                isLoading = false,
                errorMessage = null,
                stats = statsResult.getOrNull(),
                pagosProximos = pagosResult.getOrDefault(emptyList()),
                contratosCalendario = contratosResult.getOrDefault(emptyList()),
                ocupacionTendencia = ocupacionResult.getOrDefault(emptyList()),
                ingresosComparacion = ingresosResult.getOrNull(),
                gastosComparacion = gastosResult.getOrNull(),
                isFromCache = false,
                lastUpdated = null,
            )
        }
    }

    private suspend fun loadFromCache(fallbackError: String? = null) {
        val cachedStats = dashboardRepository.getCachedStats()
        val cachedPagos = dashboardRepository.getCachedPagosProximos()
        val cachedContratos = dashboardRepository.getCachedContratosCalendario()
        val cachedOcupacion = dashboardRepository.getCachedOcupacionTendencia()
        val cachedIngresos = dashboardRepository.getCachedIngresosComparacion()
        val cachedGastos = dashboardRepository.getCachedGastosComparacion()

        val hasCachedData = cachedStats != null || cachedPagos != null || cachedContratos != null

        if (!hasCachedData) {
            _uiState.update {
                it.copy(
                    isLoading = false,
                    errorMessage = fallbackError
                        ?: "Sin conexión a internet. No hay datos en caché disponibles.",
                )
            }
            return
        }

        val cachedAt = dashboardRepository.getCachedAt("stats")
        val lastUpdatedText = cachedAt?.let { formatTimestamp(it) }

        _uiState.update {
            it.copy(
                isLoading = false,
                errorMessage = null,
                stats = cachedStats,
                pagosProximos = cachedPagos ?: emptyList(),
                contratosCalendario = cachedContratos ?: emptyList(),
                ocupacionTendencia = cachedOcupacion ?: emptyList(),
                ingresosComparacion = cachedIngresos,
                gastosComparacion = cachedGastos,
                isFromCache = true,
                lastUpdated = lastUpdatedText,
            )
        }
    }

    private fun formatTimestamp(instant: Instant): String {
        val formatter = DateTimeFormatter.ofPattern("dd/MM/yyyy HH:mm")
            .withZone(ZoneId.systemDefault())
        return formatter.format(instant)
    }
}
