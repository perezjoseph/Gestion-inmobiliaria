package com.propmanager.feature.mantenimiento

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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Send
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.common.CurrencyFormatter
import com.propmanager.core.model.NotaMantenimiento
import com.propmanager.core.model.SolicitudMantenimiento
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ConfirmDeleteDialog
import com.propmanager.core.ui.components.EmptyStateScreen
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.OfflineIndicator
import com.propmanager.core.ui.components.PropManagerTextField
import com.propmanager.core.ui.components.PropManagerTopAppBar
import com.propmanager.core.ui.components.SyncStatusBadge
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MantenimientoListScreen(
    viewModel: MantenimientoViewModel,
    onNavigateToCreate: () -> Unit,
    onNavigateToDetail: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.solicitudes.collectAsStateWithLifecycle()
    val isOnline by viewModel.isOnline.collectAsStateWithLifecycle()
    val deleteTarget by viewModel.deleteTarget.collectAsStateWithLifecycle()
    val successMessage by viewModel.successMessage.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    LaunchedEffect(successMessage) { successMessage?.let { snackbarHostState.showSnackbar(it); viewModel.clearSuccessMessage() } }
    deleteTarget?.let { s -> ConfirmDeleteDialog(itemName = s.titulo, onConfirm = viewModel::confirmDelete, onDismiss = viewModel::dismissDelete) }

    Scaffold(
        topBar = { PropManagerTopAppBar(title = stringResource(R.string.mantenimiento_title), scrollBehavior = scrollBehavior) },
        floatingActionButton = { FloatingActionButton(onClick = { viewModel.initCreateForm(); onNavigateToCreate() }) { Icon(Icons.Filled.Add, contentDescription = stringResource(R.string.solicitud_create)) } },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.padding(paddingValues)) {
            OfflineIndicator(isOffline = !isOnline)
            when (val state = uiState) {
                is MantenimientoUiState.Loading -> LoadingScreen()
                is MantenimientoUiState.Success -> {
                    if (state.solicitudes.isEmpty()) {
                        EmptyStateScreen(message = stringResource(R.string.solicitud_empty))
                    } else {
                        LazyColumn(modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            item { Spacer(Modifier.height(8.dp)) }
                            items(state.solicitudes, key = { it.id }) { solicitud ->
                                SolicitudListItem(solicitud = solicitud, onClick = { onNavigateToDetail(solicitud.id) }, onDelete = { viewModel.requestDelete(solicitud) })
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
private fun SolicitudListItem(solicitud: SolicitudMantenimiento, onClick: () -> Unit, onDelete: () -> Unit, modifier: Modifier = Modifier) {
    Card(modifier = modifier.fillMaxWidth().clickable(onClick = onClick)) {
        Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(solicitud.titulo, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
                    Spacer(Modifier.width(4.dp))
                    SyncStatusBadge(isPendingSync = solicitud.isPendingSync)
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(solicitud.estado, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
                    Text(solicitud.prioridad, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                solicitud.costoMonto?.let { costo ->
                    Text(CurrencyFormatter.format(costo, solicitud.costoMoneda ?: "DOP"), style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.SemiBold)
                }
            }
            IconButton(onClick = onDelete) { Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.delete)) }
        }
    }
}
