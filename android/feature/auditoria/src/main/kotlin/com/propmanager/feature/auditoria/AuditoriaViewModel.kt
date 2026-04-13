package com.propmanager.feature.auditoria

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.propmanager.core.data.repository.AuditoriaRepository
import com.propmanager.core.network.NetworkMonitor
import com.propmanager.core.network.api.AuditoriaDto
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class AuditoriaUiState(
    val isLoading: Boolean = false,
    val errorMessage: String? = null,
    val entries: List<AuditoriaDto> = emptyList(),
    val entityTypeFilter: String? = null,
    val currentPage: Int = 1,
    val hasMore: Boolean = false,
)

@HiltViewModel
class AuditoriaViewModel
@Inject
constructor(
    private val auditoriaRepository: AuditoriaRepository,
    private val networkMonitor: NetworkMonitor,
) : ViewModel() {
    private val _uiState = MutableStateFlow(AuditoriaUiState())
    val uiState: StateFlow<AuditoriaUiState> = _uiState.asStateFlow()

    val isOnline: StateFlow<Boolean> =
        networkMonitor.isOnline.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), true)

    fun loadAuditLog(page: Int = 1) {
        viewModelScope.launch {
            _uiState.update { it.copy(isLoading = true, errorMessage = null) }
            val filters = buildMap {
                put("page", page.toString())
                put("per_page", "20")
                _uiState.value.entityTypeFilter?.let { put("entity_type", it) }
            }
            auditoriaRepository
                .fetchAuditLog(filters)
                .onSuccess { response ->
                    val newEntries =
                        if (page == 1) response.data else _uiState.value.entries + response.data
                    _uiState.update {
                        it.copy(
                            isLoading = false,
                            entries = newEntries,
                            currentPage = page,
                            hasMore = newEntries.size < response.total,
                        )
                    }
                }
                .onFailure { e ->
                    _uiState.update { it.copy(isLoading = false, errorMessage = e.message) }
                }
        }
    }

    fun setEntityTypeFilter(entityType: String?) {
        _uiState.update { it.copy(entityTypeFilter = entityType) }
        loadAuditLog(page = 1)
    }

    fun loadNextPage() {
        if (!_uiState.value.isLoading && _uiState.value.hasMore) {
            loadAuditLog(page = _uiState.value.currentPage + 1)
        }
    }
}
