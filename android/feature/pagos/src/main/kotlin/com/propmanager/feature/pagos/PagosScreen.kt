package com.propmanager.feature.pagos

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
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
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
import com.propmanager.core.common.DateFormatter
import com.propmanager.core.model.Pago
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ConfirmDeleteDialog
import com.propmanager.core.ui.components.DatePickerField
import com.propmanager.core.ui.components.EmptyStateScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.OfflineIndicator
import com.propmanager.core.ui.components.PropManagerTextField
import com.propmanager.core.ui.components.PropManagerTopAppBar
import com.propmanager.core.ui.components.SyncStatusBadge

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PagosListScreen(
    viewModel: PagosViewModel,
    onNavigateToCreate: () -> Unit,
    onNavigateToEdit: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.pagos.collectAsStateWithLifecycle()
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
    deleteTarget?.let { p ->
        ConfirmDeleteDialog(
            itemName = p.id.take(8),
            onConfirm = viewModel::confirmDelete,
            onDismiss = viewModel::dismissDelete,
        )
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.pagos_title),
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
                Icon(Icons.Filled.Add, contentDescription = stringResource(R.string.pago_create))
            }
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.padding(paddingValues)) {
            OfflineIndicator(isOffline = !isOnline)
            when (val state = uiState) {
                is PagosUiState.Loading -> LoadingScreen()
                is PagosUiState.Success -> {
                    if (state.pagos.isEmpty()) {
                        EmptyStateScreen(message = stringResource(R.string.pago_empty))
                    } else {
                        LazyColumn(
                            modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            item { Spacer(Modifier.height(8.dp)) }
                            items(state.pagos, key = { it.id }) { pago ->
                                PagoListItem(
                                    pago = pago,
                                    onClick = {
                                        viewModel.initEditForm(pago)
                                        onNavigateToEdit(pago.id)
                                    },
                                    onDelete = { viewModel.requestDelete(pago) },
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
private fun PagoListItem(
    pago: Pago,
    onClick: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth().clickable(onClick = onClick)) {
        Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        CurrencyFormatter.format(pago.monto, pago.moneda),
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.SemiBold,
                    )
                    Spacer(Modifier.width(4.dp))
                    SyncStatusBadge(isPendingSync = pago.isPendingSync)
                }
                Text(
                    "Vence: ${DateFormatter.toDisplay(pago.fechaVencimiento)}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                pago.fechaPago?.let {
                    Text(
                        "Pagado: ${DateFormatter.toDisplay(it)}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Row(
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    pago.metodoPago?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
                    Text(
                        pago.estado,
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PagoFormScreen(
    viewModel: PagosViewModel,
    isEditing: Boolean,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val formState by viewModel.formState.collectAsStateWithLifecycle()
    val contratos by viewModel.contratos.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()
    val title =
        if (isEditing) stringResource(R.string.pago_edit) else stringResource(R.string.pago_create)

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = title,
                scrollBehavior = scrollBehavior,
                onNavigateBack = onNavigateBack,
            )
        },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(
            modifier =
                Modifier.fillMaxSize()
                    .padding(paddingValues)
                    .padding(horizontal = 16.dp)
                    .verticalScroll(rememberScrollState())
        ) {
            Spacer(Modifier.height(8.dp))
            formState.errors["general"]?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                )
                Spacer(Modifier.height(8.dp))
            }

            var expanded by remember { mutableStateOf(false) }
            val selectedContrato = contratos.find { it.id == formState.contratoId }
            ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
                OutlinedTextField(
                    value =
                        selectedContrato?.let {
                            "${it.propiedadId.take(8)}… — ${DateFormatter.toDisplay(it.fechaInicio)}"
                        } ?: "",
                    onValueChange = {},
                    readOnly = true,
                    label = { Text(stringResource(R.string.pago_contrato)) },
                    trailingIcon = {
                        ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded)
                    },
                    isError = formState.errors.containsKey("contratoId"),
                    modifier = Modifier.fillMaxWidth().menuAnchor(MenuAnchorType.PrimaryNotEditable),
                )
                ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                    contratos.forEach { c ->
                        DropdownMenuItem(
                            text = {
                                Text(
                                    "${c.propiedadId.take(8)}… — ${DateFormatter.toDisplay(c.fechaInicio)}"
                                )
                            },
                            onClick = {
                                viewModel.onFieldChange("contratoId", c.id)
                                expanded = false
                            },
                        )
                    }
                }
            }
            formState.errors["contratoId"]?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(start = 16.dp, top = 4.dp),
                )
            }

            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.monto,
                onValueChange = { viewModel.onFieldChange("monto", it) },
                label = stringResource(R.string.pago_monto),
                error = formState.errors["monto"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.moneda,
                onValueChange = { viewModel.onFieldChange("moneda", it) },
                label = stringResource(R.string.pago_moneda),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            DatePickerField(
                value = formState.fechaVencimiento,
                onValueChange = viewModel::onFechaVencimientoChange,
                label = stringResource(R.string.pago_fecha_vencimiento),
                error = formState.errors["fechaVencimiento"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            DatePickerField(
                value = formState.fechaPago,
                onValueChange = viewModel::onFechaPagoChange,
                label = stringResource(R.string.pago_fecha_pago),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.metodoPago,
                onValueChange = { viewModel.onFieldChange("metodoPago", it) },
                label = stringResource(R.string.pago_metodo),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.notas,
                onValueChange = { viewModel.onFieldChange("notas", it) },
                label = stringResource(R.string.pago_notas),
                singleLine = false,
                maxLines = 3,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(16.dp))
            Button(
                onClick = { viewModel.save(onSuccess = onNavigateBack) },
                enabled = !formState.isSubmitting,
                modifier = Modifier.fillMaxWidth(),
            ) {
                if (formState.isSubmitting) CircularProgressIndicator()
                else Text(stringResource(R.string.save))
            }
            Spacer(Modifier.height(16.dp))
        }
    }
}
