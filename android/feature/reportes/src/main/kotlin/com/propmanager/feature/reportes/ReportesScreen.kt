package com.propmanager.feature.reportes

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
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.common.CurrencyFormatter
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.OfflineIndicator
import com.propmanager.core.ui.components.PropManagerTopAppBar
import java.math.BigDecimal

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReportesScreen(
    viewModel: ReportesViewModel,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val isOnline by viewModel.isOnline.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.reportes_title),
                onNavigateBack = onNavigateBack,
                scrollBehavior = scrollBehavior,
            )
        },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
            OfflineIndicator(isOffline = !isOnline)

            if (!isOnline) {
                ErrorScreen(message = stringResource(R.string.reportes_offline))
                return@Column
            }

            ReportTypeSelector(
                selected = uiState.selectedReport,
                onSelect = viewModel::selectReport,
            )

            when {
                uiState.isLoading -> LoadingScreen()
                uiState.errorMessage != null ->
                    ErrorScreen(
                        message = uiState.errorMessage!!,
                        onRetry = { viewModel.loadReport() },
                    )
                else ->
                    ReportContent(
                        uiState = uiState,
                        onExportPdf = viewModel::exportPdf,
                        onExportXlsx = viewModel::exportXlsx,
                    )
            }
        }
    }
}

@Composable
private fun ReportTypeSelector(
    selected: ReportType,
    onSelect: (ReportType) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        ReportType.entries.forEach { type ->
            FilterChip(
                selected = selected == type,
                onClick = { onSelect(type) },
                label = {
                    Text(
                        text =
                            when (type) {
                                ReportType.INGRESOS -> stringResource(R.string.reporte_ingresos)
                                ReportType.RENTABILIDAD ->
                                    stringResource(R.string.reporte_rentabilidad)
                                ReportType.HISTORIAL_PAGOS ->
                                    stringResource(R.string.reporte_historial_pagos)
                                ReportType.OCUPACION -> stringResource(R.string.reporte_ocupacion)
                            },
                        style = MaterialTheme.typography.labelSmall,
                    )
                },
            )
        }
    }
}

@Composable
private fun ReportContent(
    uiState: ReportesUiState,
    onExportPdf: () -> Unit,
    onExportXlsx: () -> Unit,
    modifier: Modifier = Modifier,
) {
    LazyColumn(
        modifier = modifier.fillMaxSize().padding(horizontal = 16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        if (
            uiState.selectedReport == ReportType.INGRESOS ||
                uiState.selectedReport == ReportType.RENTABILIDAD
        ) {
            item(key = "export_buttons") {
                ExportButtons(
                    onExportPdf = onExportPdf,
                    onExportXlsx = onExportXlsx,
                    isExporting = uiState.isExporting,
                )
            }
        }

        when (uiState.selectedReport) {
            ReportType.INGRESOS ->
                uiState.ingresos?.let { summary ->
                    items(summary.rows, key = { "${it.propiedadTitulo}-${it.inquilinoNombre}" }) {
                        row ->
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Column(modifier = Modifier.padding(12.dp)) {
                                Text(
                                    row.propiedadTitulo,
                                    style = MaterialTheme.typography.bodyLarge,
                                    fontWeight = FontWeight.Medium,
                                )
                                Text(
                                    row.inquilinoNombre,
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                ) {
                                    Text(
                                        CurrencyFormatter.format(BigDecimal(row.monto), row.moneda),
                                        style = MaterialTheme.typography.bodyMedium,
                                        fontWeight = FontWeight.SemiBold,
                                    )
                                    Text(row.estado, style = MaterialTheme.typography.bodySmall)
                                }
                            }
                        }
                    }
                }
            ReportType.RENTABILIDAD ->
                uiState.rentabilidad?.let { summary ->
                    items(summary.rows, key = { it.propiedadId }) { row ->
                        Card(modifier = Modifier.fillMaxWidth()) {
                            Column(modifier = Modifier.padding(12.dp)) {
                                Text(
                                    row.propiedadTitulo,
                                    style = MaterialTheme.typography.bodyLarge,
                                    fontWeight = FontWeight.Medium,
                                )
                                HorizontalDivider(modifier = Modifier.padding(vertical = 4.dp))
                                LabelValue(
                                    "Ingresos",
                                    CurrencyFormatter.format(
                                        BigDecimal(row.totalIngresos),
                                        row.moneda,
                                    ),
                                )
                                LabelValue(
                                    "Gastos",
                                    CurrencyFormatter.format(
                                        BigDecimal(row.totalGastos),
                                        row.moneda,
                                    ),
                                )
                                LabelValue(
                                    "Neto",
                                    CurrencyFormatter.format(
                                        BigDecimal(row.ingresoNeto),
                                        row.moneda,
                                    ),
                                )
                            }
                        }
                    }
                }
            ReportType.HISTORIAL_PAGOS -> {
                items(
                    uiState.historialPagos,
                    key = { "${it.contratoId}-${it.fechaVencimiento}" },
                ) { row ->
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(modifier = Modifier.padding(12.dp)) {
                            Text(
                                "Contrato: ${row.contratoId.take(8)}…",
                                style = MaterialTheme.typography.bodyMedium,
                            )
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                            ) {
                                Text(
                                    CurrencyFormatter.format(BigDecimal(row.monto), row.moneda),
                                    fontWeight = FontWeight.SemiBold,
                                )
                                Text(row.estado, style = MaterialTheme.typography.bodySmall)
                            }
                            Text(
                                "Vence: ${row.fechaVencimiento}",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            row.fechaPago?.let {
                                Text(
                                    "Pagado: $it",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }
            }
            ReportType.OCUPACION -> {
                items(uiState.ocupacion, key = { "${it.anio}-${it.mes}" }) { t ->
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text("${t.mes}/${t.anio}", style = MaterialTheme.typography.bodyMedium)
                        Text(
                            String.format("%.1f%%", t.tasa),
                            style = MaterialTheme.typography.bodyMedium,
                            fontWeight = FontWeight.SemiBold,
                        )
                    }
                    HorizontalDivider()
                }
            }
        }

        item(key = "bottom_spacer") { Spacer(modifier = Modifier.height(16.dp)) }
    }
}

@Composable
private fun ExportButtons(
    onExportPdf: () -> Unit,
    onExportXlsx: () -> Unit,
    isExporting: Boolean,
    modifier: Modifier = Modifier,
) {
    Row(modifier = modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(
            onClick = onExportPdf,
            enabled = !isExporting,
            modifier = Modifier.weight(1f),
        ) {
            Text(stringResource(R.string.reporte_exportar_pdf))
        }
        OutlinedButton(
            onClick = onExportXlsx,
            enabled = !isExporting,
            modifier = Modifier.weight(1f),
        ) {
            Text(stringResource(R.string.reporte_exportar_xlsx))
        }
    }
}

@Composable
private fun LabelValue(label: String, value: String, modifier: Modifier = Modifier) {
    Row(modifier = modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(
            label,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(value, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.SemiBold)
    }
}
