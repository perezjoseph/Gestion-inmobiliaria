package com.propmanager.feature.propiedades

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
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
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
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.PropManagerTopAppBar
import com.propmanager.core.ui.components.SyncStatusBadge

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
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    LaunchedEffect(propiedadId) { viewModel.loadDetail(propiedadId) }

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
                    modifier = Modifier.padding(paddingValues),
                )
        }
    }
}

@Composable
private fun PropiedadDetailContent(propiedad: Propiedad, modifier: Modifier = Modifier) {
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
