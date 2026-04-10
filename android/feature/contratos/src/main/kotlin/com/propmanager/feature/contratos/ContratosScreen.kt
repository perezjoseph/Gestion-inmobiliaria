package com.propmanager.feature.contratos

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
import com.propmanager.core.common.DateFormatter
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ConfirmDeleteDialog
import com.propmanager.core.ui.components.EmptyStateScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.OfflineIndicator
import com.propmanager.core.ui.components.PropManagerTopAppBar
import com.propmanager.core.ui.components.SyncStatusBadge

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContratosListScreen(
    viewModel: ContratosViewModel,
    onNavigateToCreate: () -> Unit,
    onNavigateToDetail: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.contratos.collectAsStateWithLifecycle()
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
    deleteTarget?.let { c ->
        ConfirmDeleteDialog(
            itemName = c.id.take(8),
            onConfirm = viewModel::confirmDelete,
            onDismiss = viewModel::dismissDelete,
        )
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.contratos_title),
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
                    contentDescription = stringResource(R.string.contrato_create),
                )
            }
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.padding(paddingValues)) {
            OfflineIndicator(isOffline = !isOnline)
            when (val state = uiState) {
                is ContratosUiState.Loading -> LoadingScreen()
                is ContratosUiState.Success -> {
                    if (state.contratos.isEmpty()) {
                        EmptyStateScreen(message = stringResource(R.string.contrato_empty))
                    } else {
                        LazyColumn(
                            modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            item { Spacer(Modifier.height(8.dp)) }
                            items(state.contratos, key = { it.contrato.id }) { cwn ->
                                ContratoListItem(
                                    cwn = cwn,
                                    onClick = { onNavigateToDetail(cwn.contrato.id) },
                                    onDelete = { viewModel.requestDelete(cwn.contrato) },
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
private fun ContratoListItem(
    cwn: ContratoWithNames,
    onClick: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val c = cwn.contrato
    Card(modifier = modifier.fillMaxWidth().clickable(onClick = onClick)) {
        Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        cwn.propiedadTitulo,
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.Medium,
                    )
                    Spacer(Modifier.width(4.dp))
                    SyncStatusBadge(isPendingSync = c.isPendingSync)
                }
                Text(
                    cwn.inquilinoNombre,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    "${DateFormatter.toDisplay(c.fechaInicio)} — ${DateFormatter.toDisplay(c.fechaFin)}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Row(
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(
                        CurrencyFormatter.format(c.montoMensual, c.moneda),
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.SemiBold,
                    )
                    Text(
                        c.estado,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
            }
            IconButton(onClick = onDelete) {
                Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.delete))
            }
        }
    }
}
