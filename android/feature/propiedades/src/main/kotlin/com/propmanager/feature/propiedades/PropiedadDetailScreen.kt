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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberModalBottomSheetState
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
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.Unidad
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ConfirmDeleteDialog
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.PropManagerTopAppBar
import com.propmanager.core.ui.components.SyncStatusBadge
import kotlinx.collections.immutable.ImmutableList

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PropiedadDetailScreen(
    viewModel: PropiedadesViewModel,
    propiedadId: String,
    onNavigateBack: () -> Unit,
    onNavigateToEdit: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val detailState by viewModel.detailState.collectAsStateWithLifecycle()
    val deleteTarget by viewModel.deleteTarget.collectAsStateWithLifecycle()
    val unidadesState by viewModel.unidadesState.collectAsStateWithLifecycle()
    val unidadDeleteTarget by viewModel.unidadDeleteTarget.collectAsStateWithLifecycle()
    val unidadFormState by viewModel.unidadFormState.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()
    var showUnidadSheet by remember { mutableStateOf(false) }

    LaunchedEffect(propiedadId) {
        viewModel.loadDetail(propiedadId)
        viewModel.loadUnidades(propiedadId)
    }

    deleteTarget?.let { propiedad ->
        ConfirmDeleteDialog(
            itemName = propiedad.titulo,
            onConfirm = {
                viewModel.confirmDelete()
                onNavigateBack()
            },
            onDismiss = viewModel::dismissDelete,
        )
    }

    unidadDeleteTarget?.let { unidad ->
        ConfirmDeleteDialog(
            itemName = unidad.numeroUnidad,
            onConfirm = viewModel::confirmDeleteUnidad,
            onDismiss = viewModel::dismissDeleteUnidad,
        )
    }

    if (showUnidadSheet) {
        UnidadFormBottomSheet(
            formState = unidadFormState,
            isEditing = unidadFormState.numeroUnidad.isNotBlank() && unidadFormState.precio.isNotBlank(),
            onFieldChange = viewModel::onUnidadFieldChange,
            onSave = { viewModel.saveUnidad { showUnidadSheet = false } },
            onDismiss = { showUnidadSheet = false },
        )
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.propiedad_detail),
                scrollBehavior = scrollBehavior,
                onNavigateBack = onNavigateBack,
                actions = {
                    if (detailState is PropiedadDetailUiState.Success) {
                        val propiedad = (detailState as PropiedadDetailUiState.Success).propiedad
                        IconButton(
                            onClick = {
                                viewModel.initEditForm(propiedad)
                                onNavigateToEdit()
                            }
                        ) {
                            Icon(
                                Icons.Filled.Edit,
                                contentDescription = stringResource(R.string.edit),
                            )
                        }
                        IconButton(onClick = { viewModel.requestDelete(propiedad) }) {
                            Icon(
                                Icons.Filled.Delete,
                                contentDescription = stringResource(R.string.delete),
                            )
                        }
                    }
                },
            )
        },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        when (val state = detailState) {
            is PropiedadDetailUiState.Loading ->
                LoadingScreen(modifier = Modifier.padding(paddingValues))
            is PropiedadDetailUiState.NotFound ->
                ErrorScreen(message = state.message, modifier = Modifier.padding(paddingValues))
            is PropiedadDetailUiState.Success ->
                PropiedadDetailContent(
                    propiedad = state.propiedad,
                    unidadesState = unidadesState,
                    onAddUnidad = {
                        viewModel.initCreateUnidadForm()
                        showUnidadSheet = true
                    },
                    onEditUnidad = { unidad ->
                        viewModel.initEditUnidadForm(unidad)
                        showUnidadSheet = true
                    },
                    onDeleteUnidad = viewModel::requestDeleteUnidad,
                    modifier = Modifier.padding(paddingValues),
                )
        }
    }
}

@Composable
private fun PropiedadDetailContent(
    propiedad: Propiedad,
    unidadesState: UnidadesUiState,
    onAddUnidad: () -> Unit,
    onEditUnidad: (Unidad) -> Unit,
    onDeleteUnidad: (Unidad) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.fillMaxSize().padding(16.dp).verticalScroll(rememberScrollState())) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(
                propiedad.titulo,
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
            )
            SyncStatusBadge(isPendingSync = propiedad.isPendingSync)
        }
        Spacer(Modifier.height(4.dp))
        Text(
            propiedad.estado,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.primary,
        )
        Spacer(Modifier.height(16.dp))
        HorizontalDivider()
        Spacer(Modifier.height(12.dp))

        DetailRow(stringResource(R.string.propiedad_direccion), propiedad.direccion)
        DetailRow(stringResource(R.string.propiedad_ciudad), propiedad.ciudad)
        DetailRow(stringResource(R.string.propiedad_provincia), propiedad.provincia)
        DetailRow(stringResource(R.string.propiedad_tipo), propiedad.tipoPropiedad)
        DetailRow(
            stringResource(R.string.propiedad_precio),
            CurrencyFormatter.format(propiedad.precio, propiedad.moneda),
        )
        propiedad.habitaciones?.let {
            DetailRow(stringResource(R.string.propiedad_habitaciones), it.toString())
        }
        propiedad.banos?.let { DetailRow(stringResource(R.string.propiedad_banos), it.toString()) }
        propiedad.areaM2?.let {
            DetailRow(stringResource(R.string.propiedad_area), it.toPlainString())
        }
        propiedad.descripcion?.let {
            Spacer(Modifier.height(8.dp))
            Text(
                stringResource(R.string.propiedad_descripcion),
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(it, style = MaterialTheme.typography.bodyMedium)
        }

        Spacer(Modifier.height(24.dp))
        HorizontalDivider()
        Spacer(Modifier.height(16.dp))

        UnidadesSection(
            unidadesState = unidadesState,
            onAddUnidad = onAddUnidad,
            onEditUnidad = onEditUnidad,
            onDeleteUnidad = onDeleteUnidad,
        )
    }
}

@Composable
private fun UnidadesSection(
    unidadesState: UnidadesUiState,
    onAddUnidad: () -> Unit,
    onEditUnidad: (Unidad) -> Unit,
    onDeleteUnidad: (Unidad) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                stringResource(R.string.unidades_title),
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
            )
            IconButton(onClick = onAddUnidad) {
                Icon(
                    Icons.Filled.Add,
                    contentDescription = stringResource(R.string.unidad_create),
                )
            }
        }
        Spacer(Modifier.height(8.dp))

        when (unidadesState) {
            is UnidadesUiState.Loading -> {
                CircularProgressIndicator(modifier = Modifier.align(Alignment.CenterHorizontally))
            }
            is UnidadesUiState.Error -> {
                Text(
                    unidadesState.message,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.error,
                )
            }
            is UnidadesUiState.Success -> {
                if (unidadesState.unidades.isEmpty()) {
                    Text(
                        stringResource(R.string.unidad_empty),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    UnidadesList(
                        unidades = unidadesState.unidades,
                        onEdit = onEditUnidad,
                        onDelete = onDeleteUnidad,
                    )
                }
            }
        }
    }
}

@Composable
private fun UnidadesList(
    unidades: ImmutableList<Unidad>,
    onEdit: (Unidad) -> Unit,
    onDelete: (Unidad) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        unidades.forEach { unidad ->
            UnidadListItem(
                unidad = unidad,
                onEdit = { onEdit(unidad) },
                onDelete = { onDelete(unidad) },
            )
        }
    }
}

@Composable
private fun UnidadListItem(
    unidad: Unidad,
    onEdit: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth().clickable(onClick = onEdit)) {
        Row(
            modifier = Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    unidad.numeroUnidad,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    unidad.piso?.let {
                        Text(
                            "${stringResource(R.string.unidad_piso)}: $it",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Text(
                        unidad.estado,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
                Text(
                    CurrencyFormatter.format(unidad.precio, unidad.moneda),
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.SemiBold,
                )
            }
            IconButton(onClick = onEdit) {
                Icon(Icons.Filled.Edit, contentDescription = stringResource(R.string.edit))
            }
            IconButton(onClick = onDelete) {
                Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.delete))
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun UnidadFormBottomSheet(
    formState: UnidadFormState,
    isEditing: Boolean,
    onFieldChange: (String, String) -> Unit,
    onSave: () -> Unit,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        modifier = modifier,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp)
                .padding(bottom = 32.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(
                text = if (isEditing) stringResource(R.string.unidad_edit) else stringResource(R.string.unidad_create),
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold,
            )

            formState.errors["general"]?.let { error ->
                Text(
                    error,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }

            OutlinedTextField(
                value = formState.numeroUnidad,
                onValueChange = { onFieldChange("numeroUnidad", it) },
                label = { Text(stringResource(R.string.unidad_numero)) },
                isError = formState.errors.containsKey("numeroUnidad"),
                supportingText = formState.errors["numeroUnidad"]?.let { { Text(it) } },
                modifier = Modifier.fillMaxWidth(),
            )

            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = formState.piso,
                    onValueChange = { onFieldChange("piso", it) },
                    label = { Text(stringResource(R.string.unidad_piso)) },
                    modifier = Modifier.weight(1f),
                )
                OutlinedTextField(
                    value = formState.habitaciones,
                    onValueChange = { onFieldChange("habitaciones", it) },
                    label = { Text(stringResource(R.string.unidad_habitaciones)) },
                    modifier = Modifier.weight(1f),
                )
            }

            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = formState.banos,
                    onValueChange = { onFieldChange("banos", it) },
                    label = { Text(stringResource(R.string.unidad_banos)) },
                    modifier = Modifier.weight(1f),
                )
                OutlinedTextField(
                    value = formState.areaM2,
                    onValueChange = { onFieldChange("areaM2", it) },
                    label = { Text(stringResource(R.string.unidad_area)) },
                    modifier = Modifier.weight(1f),
                )
            }

            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = formState.precio,
                    onValueChange = { onFieldChange("precio", it) },
                    label = { Text(stringResource(R.string.unidad_precio)) },
                    isError = formState.errors.containsKey("precio"),
                    supportingText = formState.errors["precio"]?.let { { Text(it) } },
                    modifier = Modifier.weight(1f),
                )
                OutlinedTextField(
                    value = formState.moneda,
                    onValueChange = { onFieldChange("moneda", it) },
                    label = { Text(stringResource(R.string.unidad_moneda)) },
                    modifier = Modifier.weight(1f),
                )
            }

            OutlinedTextField(
                value = formState.estado,
                onValueChange = { onFieldChange("estado", it) },
                label = { Text(stringResource(R.string.unidad_estado)) },
                modifier = Modifier.fillMaxWidth(),
            )

            OutlinedTextField(
                value = formState.descripcion,
                onValueChange = { onFieldChange("descripcion", it) },
                label = { Text(stringResource(R.string.unidad_descripcion)) },
                modifier = Modifier.fillMaxWidth(),
                minLines = 2,
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                TextButton(onClick = onDismiss) {
                    Text(stringResource(R.string.cancel))
                }
                Spacer(Modifier.width(8.dp))
                TextButton(
                    onClick = onSave,
                    enabled = !formState.isSubmitting,
                ) {
                    if (formState.isSubmitting) {
                        CircularProgressIndicator(
                            modifier = Modifier.height(16.dp).width(16.dp),
                            strokeWidth = 2.dp,
                        )
                    } else {
                        Text(stringResource(R.string.save))
                    }
                }
            }
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
