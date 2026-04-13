package com.propmanager.feature.mantenimiento

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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Send
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import com.propmanager.core.model.NotaMantenimiento
import com.propmanager.core.model.SolicitudMantenimiento
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ConfirmDeleteDialog
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.PropManagerTextField
import com.propmanager.core.ui.components.PropManagerTopAppBar
import com.propmanager.core.ui.components.SyncStatusBadge
import java.time.ZoneId
import java.time.format.DateTimeFormatter

private val displayFormatter =
    DateTimeFormatter.ofPattern("dd/MM/yyyy HH:mm").withZone(ZoneId.systemDefault())

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SolicitudDetailScreen(
    viewModel: MantenimientoViewModel,
    solicitudId: String,
    onNavigateBack: () -> Unit,
    onNavigateToEdit: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val detailState by viewModel.detailState.collectAsStateWithLifecycle()
    val notaInput by viewModel.notaInput.collectAsStateWithLifecycle()
    val deleteTarget by viewModel.deleteTarget.collectAsStateWithLifecycle()
    val showEstado by viewModel.showEstadoDialog.collectAsStateWithLifecycle()
    val successMessage by viewModel.successMessage.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    LaunchedEffect(solicitudId) { viewModel.loadDetail(solicitudId) }
    LaunchedEffect(successMessage) {
        successMessage?.let {
            snackbarHostState.showSnackbar(it)
            viewModel.clearSuccessMessage()
        }
    }

    deleteTarget?.let { s ->
        ConfirmDeleteDialog(
            itemName = s.titulo,
            onConfirm = {
                viewModel.confirmDelete()
                onNavigateBack()
            },
            onDismiss = viewModel::dismissDelete,
        )
    }

    if (showEstado) {
        val estados = listOf("pendiente", "en_progreso", "completada", "cancelada")
        AlertDialog(
            onDismissRequest = viewModel::dismissEstadoChange,
            title = { Text(stringResource(R.string.solicitud_cambiar_estado)) },
            text = {
                Column {
                    estados.forEach { estado ->
                        TextButton(
                            onClick = { viewModel.changeEstado(estado) },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(estado)
                        }
                    }
                }
            },
            confirmButton = {},
            dismissButton = {
                TextButton(onClick = viewModel::dismissEstadoChange) {
                    Text(stringResource(R.string.cancel))
                }
            },
        )
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.solicitud_detail),
                scrollBehavior = scrollBehavior,
                onNavigateBack = onNavigateBack,
                actions = {
                    if (detailState is SolicitudDetailUiState.Success) {
                        val s = (detailState as SolicitudDetailUiState.Success).solicitud
                        IconButton(
                            onClick = {
                                viewModel.initEditForm(s)
                                onNavigateToEdit()
                            }
                        ) {
                            Icon(
                                Icons.Filled.Edit,
                                contentDescription = stringResource(R.string.edit),
                            )
                        }
                        IconButton(onClick = { viewModel.requestDelete(s) }) {
                            Icon(
                                Icons.Filled.Delete,
                                contentDescription = stringResource(R.string.delete),
                            )
                        }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        when (val state = detailState) {
            is SolicitudDetailUiState.Loading ->
                LoadingScreen(modifier = Modifier.padding(paddingValues))
            is SolicitudDetailUiState.NotFound ->
                ErrorScreen(message = state.message, modifier = Modifier.padding(paddingValues))
            is SolicitudDetailUiState.Success -> {
                LazyColumn(
                    modifier =
                        Modifier.fillMaxSize().padding(paddingValues).padding(horizontal = 16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    item { SolicitudHeader(state.solicitud) }
                    item { SolicitudFields(state.solicitud) }
                    item {
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Button(
                                onClick = viewModel::showEstadoChange,
                                modifier = Modifier.weight(1f),
                            ) {
                                Text(stringResource(R.string.solicitud_cambiar_estado))
                            }
                        }
                    }
                    item {
                        HorizontalDivider()
                        Spacer(Modifier.height(8.dp))
                        Text(
                            stringResource(R.string.solicitud_notas),
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.SemiBold,
                        )
                    }
                    items(state.notas, key = { it.id }) { nota -> NotaItem(nota) }
                    item {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            PropManagerTextField(
                                value = notaInput,
                                onValueChange = viewModel::onNotaInputChange,
                                label = stringResource(R.string.solicitud_agregar_nota),
                                modifier = Modifier.weight(1f),
                            )
                            IconButton(onClick = viewModel::addNota) {
                                Icon(
                                    Icons.Filled.Send,
                                    contentDescription =
                                        stringResource(R.string.solicitud_agregar_nota),
                                )
                            }
                        }
                    }
                    item { Spacer(Modifier.height(16.dp)) }
                }
            }
        }
    }
}

@Composable
private fun SolicitudHeader(solicitud: SolicitudMantenimiento) {
    Spacer(Modifier.height(8.dp))
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            solicitud.titulo,
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
        )
        SyncStatusBadge(isPendingSync = solicitud.isPendingSync)
    }
    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(
            solicitud.estado,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.primary,
        )
        Text(
            solicitud.prioridad,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun SolicitudFields(solicitud: SolicitudMantenimiento) {
    HorizontalDivider()
    Spacer(Modifier.height(8.dp))
    solicitud.descripcion?.let { DetailRow(stringResource(R.string.solicitud_descripcion), it) }
    solicitud.nombreProveedor?.let {
        DetailRow(stringResource(R.string.solicitud_proveedor_nombre), it)
    }
    solicitud.telefonoProveedor?.let {
        DetailRow(stringResource(R.string.solicitud_proveedor_telefono), it)
    }
    solicitud.emailProveedor?.let {
        DetailRow(stringResource(R.string.solicitud_proveedor_email), it)
    }
    solicitud.costoMonto?.let { costo ->
        DetailRow(
            stringResource(R.string.solicitud_costo),
            CurrencyFormatter.format(costo, solicitud.costoMoneda ?: "DOP"),
        )
    }
}

@Composable
private fun NotaItem(nota: NotaMantenimiento) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(nota.contenido, style = MaterialTheme.typography.bodyMedium)
            Text(
                displayFormatter.format(nota.createdAt),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun DetailRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(
            label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(value, style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.Medium)
    }
}
