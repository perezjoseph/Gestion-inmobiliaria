package com.propmanager.feature.propiedades

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.common.CurrencyFormatter
import com.propmanager.core.model.Propiedad
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ConfirmDeleteDialog
import com.propmanager.core.ui.components.EmptyStateScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.OfflineIndicator
import com.propmanager.core.ui.components.PropManagerTopAppBar
import com.propmanager.core.ui.components.SyncStatusBadge

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PropiedadesListScreen(
    viewModel: PropiedadesViewModel,
    onNavigateToCreate: () -> Unit,
    onNavigateToDetail: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.propiedades.collectAsStateWithLifecycle()
    val isOnline by viewModel.isOnline.collectAsStateWithLifecycle()
    val deleteTarget by viewModel.deleteTarget.collectAsStateWithLifecycle()
    val successMessage by viewModel.successMessage.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    LaunchedEffect(successMessage) {
        successMessage?.let {
            snackbarHostState.showSnackbar(it)
            viewModel.clearSuccessMessage()
        }
    }

    deleteTarget?.let { propiedad ->
        ConfirmDeleteDialog(
            itemName = propiedad.titulo,
            onConfirm = viewModel::confirmDelete,
            onDismiss = viewModel::dismissDelete,
        )
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.propiedades_title),
                scrollBehavior = scrollBehavior,
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = {
                    viewModel.initCreateForm()
                    onNavigateToCreate()
                }
            ) {
                Icon(
                    Icons.Filled.Add,
                    contentDescription = stringResource(R.string.propiedad_create),
                )
            }
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.padding(paddingValues)) {
            OfflineIndicator(isOffline = !isOnline)
            when (val state = uiState) {
                is PropiedadesUiState.Loading -> LoadingScreen()
                is PropiedadesUiState.Error -> EmptyStateScreen(message = state.message)
                is PropiedadesUiState.Success -> {
                    if (state.propiedades.isEmpty()) {
                        EmptyStateScreen(message = stringResource(R.string.propiedad_empty))
                    } else {
                        LazyColumn(
                            modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            item { Spacer(Modifier.height(8.dp)) }
                            items(state.propiedades, key = { it.id }) { propiedad ->
                                PropiedadListItem(
                                    propiedad = propiedad,
                                    onClick = { onNavigateToDetail(propiedad.id) },
                                    onDelete = { viewModel.requestDelete(propiedad) },
                                )
                            }
                            item { Spacer(Modifier.height(80.dp)) }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun PropiedadListItem(
    propiedad: Propiedad,
    onClick: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth().clickable(onClick = onClick)) {
        Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        propiedad.titulo,
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.Medium,
                    )
                    Spacer(Modifier.width(4.dp))
                    SyncStatusBadge(isPendingSync = propiedad.isPendingSync)
                }
                Text(
                    "${propiedad.ciudad} · ${propiedad.tipoPropiedad}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    CurrencyFormatter.format(propiedad.precio, propiedad.moneda),
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    propiedad.estado,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            IconButton(onClick = onDelete) {
                Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.delete))
            }
        }
    }
}
