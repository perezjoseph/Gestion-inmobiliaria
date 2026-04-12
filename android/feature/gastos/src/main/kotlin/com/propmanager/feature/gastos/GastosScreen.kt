package com.propmanager.feature.gastos

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
import androidx.compose.material3.OutlinedButton
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
import com.propmanager.core.model.Gasto
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
fun GastosListScreen(
    viewModel: GastosViewModel,
    onNavigateToCreate: () -> Unit,
    onNavigateToEdit: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.gastos.collectAsStateWithLifecycle()
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
    deleteTarget?.let { g ->
        ConfirmDeleteDialog(itemName = g.descripcion, onConfirm = viewModel::confirmDelete, onDismiss = viewModel::dismissDelete)
    }

    Scaffold(
        topBar = { PropManagerTopAppBar(title = stringResource(R.string.gastos_title), scrollBehavior = scrollBehavior) },
        floatingActionButton = {
            FloatingActionButton(onClick = {
                viewModel.initCreateForm()
                onNavigateToCreate()
            }) { Icon(Icons.Filled.Add, contentDescription = stringResource(R.string.gasto_create)) }
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.padding(paddingValues)) {
            OfflineIndicator(isOffline = !isOnline)
            when (val state = uiState) {
                is GastosUiState.Loading -> LoadingScreen()
                is GastosUiState.Success -> {
                    if (state.gastos.isEmpty()) {
                        EmptyStateScreen(message = stringResource(R.string.gasto_empty))
                    } else {
                        LazyColumn(
                            modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            item { Spacer(Modifier.height(8.dp)) }
                            items(state.gastos, key = { it.id }) { gasto ->
                                GastoListItem(gasto = gasto, onClick = {
                                    viewModel.initEditForm(gasto)
                                    onNavigateToEdit(gasto.id)
                                }, onDelete = { viewModel.requestDelete(gasto) })
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
private fun GastoListItem(
    gasto: Gasto,
    onClick: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth().clickable(onClick = onClick)) {
        Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(gasto.categoria, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
                    Spacer(Modifier.width(4.dp))
                    SyncStatusBadge(isPendingSync = gasto.isPendingSync)
                }
                Text(
                    gasto.descripcion,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                )
                Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                    Text(
                        CurrencyFormatter.format(gasto.monto, gasto.moneda),
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.SemiBold,
                    )
                    Text(
                        DateFormatter.toDisplay(gasto.fechaGasto),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Text(gasto.estado, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary)
            }
            IconButton(onClick = onDelete) { Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.delete)) }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GastoFormScreen(
    viewModel: GastosViewModel,
    isEditing: Boolean,
    onNavigateBack: () -> Unit,
    onScanRecibo: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val formState by viewModel.formState.collectAsStateWithLifecycle()
    val propiedades by viewModel.propiedades.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()
    val title = if (isEditing) stringResource(R.string.gasto_edit) else stringResource(R.string.gasto_create)

    Scaffold(
        topBar = {
            PropManagerTopAppBar(title = title, scrollBehavior = scrollBehavior, onNavigateBack = onNavigateBack)
        },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(paddingValues)
                    .padding(horizontal = 16.dp)
                    .verticalScroll(rememberScrollState()),
        ) {
            Spacer(Modifier.height(8.dp))
            formState.errors["general"]?.let {
                Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                Spacer(Modifier.height(8.dp))
            }

            var propExpanded by remember { mutableStateOf(false) }
            val selectedProp = propiedades.find { it.id == formState.propiedadId }
            ExposedDropdownMenuBox(expanded = propExpanded, onExpandedChange = { propExpanded = it }) {
                OutlinedTextField(
                    value = selectedProp?.titulo ?: "",
                    onValueChange = {},
                    readOnly = true,
                    label = { Text(stringResource(R.string.gasto_propiedad)) },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = propExpanded) },
                    isError = formState.errors.containsKey("propiedadId"),
                    modifier = Modifier.fillMaxWidth().menuAnchor(MenuAnchorType.PrimaryNotEditable),
                )
                ExposedDropdownMenu(expanded = propExpanded, onDismissRequest = { propExpanded = false }) {
                    propiedades.forEach { p ->
                        DropdownMenuItem(text = { Text(p.titulo) }, onClick = {
                            viewModel.onFieldChange("propiedadId", p.id)
                            propExpanded =
                                false
                        })
                    }
                }
            }
            formState.errors["propiedadId"]?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(start = 16.dp, top = 4.dp),
                )
            }

            Spacer(Modifier.height(8.dp))

            var catExpanded by remember { mutableStateOf(false) }
            val categorias = listOf("mantenimiento", "reparacion", "servicios", "impuestos", "seguros", "administracion", "otro")
            ExposedDropdownMenuBox(expanded = catExpanded, onExpandedChange = { catExpanded = it }) {
                OutlinedTextField(
                    value = formState.categoria,
                    onValueChange = {},
                    readOnly = true,
                    label = { Text(stringResource(R.string.gasto_categoria)) },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = catExpanded) },
                    isError = formState.errors.containsKey("categoria"),
                    modifier = Modifier.fillMaxWidth().menuAnchor(MenuAnchorType.PrimaryNotEditable),
                )
                ExposedDropdownMenu(expanded = catExpanded, onDismissRequest = { catExpanded = false }) {
                    categorias.forEach { cat ->
                        DropdownMenuItem(text = { Text(cat) }, onClick = {
                            viewModel.onFieldChange("categoria", cat)
                            catExpanded =
                                false
                        })
                    }
                }
            }
            formState.errors["categoria"]?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(start = 16.dp, top = 4.dp),
                )
            }

            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.descripcion,
                onValueChange = {
                    viewModel.onFieldChange("descripcion", it)
                },
                label =
                    stringResource(
                        R.string.gasto_descripcion,
                    ),
                error = formState.errors["descripcion"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.monto, onValueChange = {
                viewModel.onFieldChange("monto", it)
            }, label = stringResource(R.string.gasto_monto), error = formState.errors["monto"], modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.moneda, onValueChange = {
                viewModel.onFieldChange("moneda", it)
            }, label = stringResource(R.string.gasto_moneda), error = formState.errors["moneda"], modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            DatePickerField(
                value = formState.fechaGasto,
                onValueChange = viewModel::onFechaGastoChange,
                label = stringResource(R.string.gasto_fecha),
                error = formState.errors["fechaGasto"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.proveedor, onValueChange = {
                viewModel.onFieldChange("proveedor", it)
            }, label = stringResource(R.string.gasto_proveedor), modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.numeroFactura, onValueChange = {
                viewModel.onFieldChange("numeroFactura", it)
            }, label = stringResource(R.string.gasto_numero_factura), modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            OutlinedButton(onClick = onScanRecibo, modifier = Modifier.fillMaxWidth()) { Text(stringResource(R.string.gasto_scan_recibo)) }
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.notas, onValueChange = {
                viewModel.onFieldChange("notas", it)
            }, label = stringResource(R.string.gasto_notas), singleLine = false, maxLines = 3, modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(16.dp))
            Button(
                onClick = { viewModel.save(onSuccess = onNavigateBack) },
                enabled = !formState.isSubmitting,
                modifier = Modifier.fillMaxWidth(),
            ) {
                if (formState.isSubmitting) CircularProgressIndicator() else Text(stringResource(R.string.save))
            }
            Spacer(Modifier.height(16.dp))
        }
    }
}
