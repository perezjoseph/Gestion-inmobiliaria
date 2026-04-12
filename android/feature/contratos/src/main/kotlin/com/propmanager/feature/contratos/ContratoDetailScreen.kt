package com.propmanager.feature.contratos

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import com.propmanager.core.ui.components.DatePickerField
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.PropManagerTextField
import com.propmanager.core.ui.components.PropManagerTopAppBar
import com.propmanager.core.ui.components.SyncStatusBadge

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContratoDetailScreen(
    viewModel: ContratosViewModel,
    contratoId: String,
    onNavigateBack: () -> Unit,
    onNavigateToEdit: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val detailState by viewModel.detailState.collectAsStateWithLifecycle()
    val deleteTarget by viewModel.deleteTarget.collectAsStateWithLifecycle()
    val showRenew by viewModel.showRenewDialog.collectAsStateWithLifecycle()
    val showTerminate by viewModel.showTerminateDialog.collectAsStateWithLifecycle()
    val renewForm by viewModel.renewForm.collectAsStateWithLifecycle()
    val successMessage by viewModel.successMessage.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    LaunchedEffect(contratoId) { viewModel.loadDetail(contratoId) }
    LaunchedEffect(successMessage) { successMessage?.let { snackbarHostState.showSnackbar(it); viewModel.clearSuccessMessage() } }

    deleteTarget?.let { c ->
        ConfirmDeleteDialog(itemName = c.id.take(8), onConfirm = { viewModel.confirmDelete(); onNavigateBack() }, onDismiss = viewModel::dismissDelete)
    }

    if (showRenew) {
        AlertDialog(
            onDismissRequest = viewModel::dismissRenew,
            title = { Text(stringResource(R.string.contrato_renovar)) },
            text = {
                Column {
                    DatePickerField(value = renewForm.fechaFin, onValueChange = viewModel::onRenewFechaFinChange, label = stringResource(R.string.contrato_fecha_fin), error = renewForm.errors["fechaFin"])
                    Spacer(Modifier.height(8.dp))
                    PropManagerTextField(value = renewForm.montoMensual, onValueChange = viewModel::onRenewMontoChange, label = stringResource(R.string.contrato_monto_mensual), error = renewForm.errors["montoMensual"])
                }
            },
            confirmButton = { TextButton(onClick = viewModel::confirmRenew) { Text(stringResource(R.string.confirm)) } },
            dismissButton = { TextButton(onClick = viewModel::dismissRenew) { Text(stringResource(R.string.cancel)) } },
        )
    }

    if (showTerminate) {
        AlertDialog(
            onDismissRequest = viewModel::dismissTerminate,
            title = { Text(stringResource(R.string.contrato_terminar)) },
            text = { Text(stringResource(R.string.contrato_fecha_terminacion) + ": " + DateFormatter.toDisplay(java.time.LocalDate.now())) },
            confirmButton = { TextButton(onClick = viewModel::confirmTerminate) { Text(stringResource(R.string.confirm)) } },
            dismissButton = { TextButton(onClick = viewModel::dismissTerminate) { Text(stringResource(R.string.cancel)) } },
        )
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.contrato_detail),
                scrollBehavior = scrollBehavior,
                navigationIcon = { IconButton(onClick = onNavigateBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = stringResource(R.string.navigate_back)) } },
                actions = {
                    if (detailState is ContratoDetailUiState.Success) {
                        val c = (detailState as ContratoDetailUiState.Success).contrato.contrato
                        IconButton(onClick = { viewModel.initEditForm(c); onNavigateToEdit() }) { Icon(Icons.Filled.Edit, contentDescription = stringResource(R.string.edit)) }
                        IconButton(onClick = { viewModel.requestDelete(c) }) { Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.delete)) }
                    }
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        when (val state = detailState) {
            is ContratoDetailUiState.Loading -> LoadingScreen(modifier = Modifier.padding(paddingValues))
            is ContratoDetailUiState.NotFound -> ErrorScreen(message = state.message, modifier = Modifier.padding(paddingValues))
            is ContratoDetailUiState.Success -> ContratoDetailContent(
                cwn = state.contrato,
                onRenew = viewModel::showRenew,
                onTerminate = viewModel::showTerminate,
                modifier = Modifier.padding(paddingValues),
            )
        }
    }
}

@Composable
private fun ContratoDetailContent(cwn: ContratoWithNames, onRenew: () -> Unit, onTerminate: () -> Unit, modifier: Modifier = Modifier) {
    val c = cwn.contrato
    Column(modifier = modifier.fillMaxSize().padding(16.dp).verticalScroll(rememberScrollState())) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(cwn.propiedadTitulo, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
            SyncStatusBadge(isPendingSync = c.isPendingSync)
        }
        Text(cwn.inquilinoNombre, style = MaterialTheme.typography.bodyLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(c.estado, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.primary)
        Spacer(Modifier.height(16.dp))
        HorizontalDivider()
        Spacer(Modifier.height(12.dp))
        DetailRow(stringResource(R.string.contrato_fecha_inicio), DateFormatter.toDisplay(c.fechaInicio))
        DetailRow(stringResource(R.string.contrato_fecha_fin), DateFormatter.toDisplay(c.fechaFin))
        DetailRow(stringResource(R.string.contrato_monto_mensual), CurrencyFormatter.format(c.montoMensual, c.moneda))
        c.deposito?.let { DetailRow(stringResource(R.string.contrato_deposito), CurrencyFormatter.format(it, c.moneda)) }
        DetailRow(stringResource(R.string.contrato_moneda), c.moneda)
        Spacer(Modifier.height(16.dp))
        if (c.estado == "activo") {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                Button(onClick = onRenew, modifier = Modifier.weight(1f)) { Text(stringResource(R.string.contrato_renovar)) }
                OutlinedButton(onClick = onTerminate, modifier = Modifier.weight(1f)) { Text(stringResource(R.string.contrato_terminar)) }
            }
        }
    }
}

@Composable
private fun DetailRow(label: String, value: String) {
    Row(modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(label, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.Medium)
    }
}
