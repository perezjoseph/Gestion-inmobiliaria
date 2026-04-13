package com.propmanager.feature.auditoria

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.network.api.AuditoriaDto
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.EmptyStateScreen
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.OfflineIndicator
import com.propmanager.core.ui.components.PropManagerTopAppBar

private val ENTITY_TYPES =
    listOf("propiedad", "inquilino", "contrato", "pago", "gasto", "solicitud")

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AuditoriaScreen(
    viewModel: AuditoriaViewModel,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val isOnline by viewModel.isOnline.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.auditoria_title),
                onNavigateBack = onNavigateBack,
                scrollBehavior = scrollBehavior,
            )
        },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
            OfflineIndicator(isOffline = !isOnline)

            if (!isOnline) {
                ErrorScreen(message = stringResource(R.string.auditoria_offline))
                return@Column
            }

            EntityTypeFilterRow(
                selected = uiState.entityTypeFilter,
                onSelect = viewModel::setEntityTypeFilter,
            )

            when {
                uiState.isLoading && uiState.entries.isEmpty() -> LoadingScreen()
                uiState.errorMessage != null && uiState.entries.isEmpty() ->
                    ErrorScreen(
                        message = uiState.errorMessage!!,
                        onRetry = { viewModel.loadAuditLog() },
                    )
                uiState.entries.isEmpty() ->
                    EmptyStateScreen(message = stringResource(R.string.auditoria_empty))
                else -> {
                    val listState = rememberLazyListState()
                    val shouldLoadMore by remember {
                        derivedStateOf {
                            val lastVisible =
                                listState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
                            lastVisible >= uiState.entries.size - 3
                        }
                    }
                    LaunchedEffect(shouldLoadMore) { if (shouldLoadMore) viewModel.loadNextPage() }

                    LazyColumn(
                        state = listState,
                        modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        item(key = "top_spacer") { Spacer(modifier = Modifier.height(4.dp)) }
                        items(uiState.entries, key = { it.id }) { entry ->
                            AuditoriaItem(entry = entry)
                        }
                        if (uiState.isLoading) {
                            item(key = "loading_more") {
                                Text(
                                    text = stringResource(R.string.loading),
                                    modifier = Modifier.fillMaxWidth().padding(16.dp),
                                    style = MaterialTheme.typography.bodySmall,
                                )
                            }
                        }
                        item(key = "bottom_spacer") { Spacer(modifier = Modifier.height(16.dp)) }
                    }
                }
            }
        }
    }
}

@Composable
private fun EntityTypeFilterRow(
    selected: String?,
    onSelect: (String?) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        FilterChip(
            selected = selected == null,
            onClick = { onSelect(null) },
            label = { Text("Todos", style = MaterialTheme.typography.labelSmall) },
        )
        ENTITY_TYPES.forEach { type ->
            FilterChip(
                selected = selected == type,
                onClick = { onSelect(type) },
                label = {
                    Text(
                        type.replaceFirstChar { it.uppercase() },
                        style = MaterialTheme.typography.labelSmall,
                    )
                },
            )
        }
    }
}

@Composable
private fun AuditoriaItem(entry: AuditoriaDto, modifier: Modifier = Modifier) {
    Card(modifier = modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    text = entry.accion,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium,
                )
                Text(
                    text = entry.entityType,
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
            }
            Text(
                text = "ID: ${entry.entityId.take(8)}…",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                text = entry.createdAt,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
