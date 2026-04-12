package com.propmanager.feature.dashboard

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.common.CurrencyFormatter
import com.propmanager.core.model.dto.ContratoCalendario
import com.propmanager.core.model.dto.GastosComparacion
import com.propmanager.core.model.dto.IngresosComparacion
import com.propmanager.core.model.dto.OcupacionTendencia
import com.propmanager.core.model.dto.PagoProximo
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.OfflineIndicator
import com.propmanager.core.ui.components.PropManagerTopAppBar
import java.math.BigDecimal

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    viewModel: DashboardViewModel,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val isOnline by viewModel.isOnline.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.dashboard_title),
                scrollBehavior = scrollBehavior,
                actions = {
                    IconButton(onClick = viewModel::loadDashboard) {
                        Icon(
                            imageVector = Icons.Filled.Refresh,
                            contentDescription = stringResource(R.string.retry),
                        )
                    }
                },
            )
        },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues),
        ) {
            OfflineIndicator(isOffline = !isOnline)

            when {
                uiState.isLoading -> LoadingScreen()
                uiState.errorMessage != null && uiState.stats == null -> {
                    ErrorScreen(
                        message = uiState.errorMessage!!,
                        onRetry = viewModel::loadDashboard,
                    )
                }
                else -> DashboardContent(uiState = uiState)
            }
        }
    }
}

@Composable
private fun DashboardContent(
    uiState: DashboardUiState,
    modifier: Modifier = Modifier,
) {
    LazyColumn(
        modifier = modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        if (uiState.isFromCache && uiState.lastUpdated != null) {
            item(key = "staleness") {
                StalenessIndicator(lastUpdated = uiState.lastUpdated)
            }
        }

        uiState.stats?.let { stats ->
            item(key = "stats") {
                Spacer(modifier = Modifier.height(8.dp))
                StatsGrid(stats = stats)
            }
        }

        if (uiState.pagosProximos.isNotEmpty()) {
            item(key = "pagos_header") {
                SectionHeader(title = stringResource(R.string.dashboard_pagos_proximos))
            }
            items(
                items = uiState.pagosProximos,
                key = { it.pagoId },
            ) { pago ->
                PagoProximoItem(pago = pago)
            }
        }

        if (uiState.contratosCalendario.isNotEmpty()) {
            item(key = "contratos_header") {
                SectionHeader(title = stringResource(R.string.dashboard_contratos_por_vencer))
            }
            items(
                items = uiState.contratosCalendario,
                key = { it.contratoId },
            ) { contrato ->
                ContratoCalendarioItem(contrato = contrato)
            }
        }

        if (uiState.ocupacionTendencia.isNotEmpty()) {
            item(key = "ocupacion_header") {
                SectionHeader(title = stringResource(R.string.dashboard_ocupacion))
            }
            items(
                items = uiState.ocupacionTendencia,
                key = { "${it.anio}-${it.mes}" },
            ) { tendencia ->
                OcupacionTendenciaItem(tendencia = tendencia)
            }
        }

        uiState.ingresosComparacion?.let { ingresos ->
            item(key = "ingresos") {
                SectionHeader(title = stringResource(R.string.dashboard_ingresos))
                Spacer(modifier = Modifier.height(8.dp))
                IngresosComparacionCard(ingresos = ingresos)
            }
        }

        uiState.gastosComparacion?.let { gastos ->
            item(key = "gastos") {
                SectionHeader(title = stringResource(R.string.dashboard_gastos))
                Spacer(modifier = Modifier.height(8.dp))
                GastosComparacionCard(gastos = gastos)
            }
        }

        item(key = "bottom_spacer") {
            Spacer(modifier = Modifier.height(16.dp))
        }
    }
}

@Composable
private fun StalenessIndicator(
    lastUpdated: String,
    modifier: Modifier = Modifier,
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.tertiaryContainer,
        ),
    ) {
        Text(
            text = stringResource(R.string.last_updated, lastUpdated),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onTertiaryContainer,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
        )
    }
}

@Composable
private fun StatsGrid(
    stats: com.propmanager.core.model.dto.DashboardStats,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            StatCard(
                label = stringResource(R.string.dashboard_total_propiedades),
                value = stats.totalPropiedades.toString(),
                modifier = Modifier.weight(1f),
            )
            StatCard(
                label = stringResource(R.string.dashboard_contratos_activos),
                value = String.format("%.0f%%", stats.tasaOcupacion),
                modifier = Modifier.weight(1f),
            )
        }
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            StatCard(
                label = stringResource(R.string.dashboard_pagos_pendientes),
                value = stats.pagosAtrasados.toString(),
                modifier = Modifier.weight(1f),
            )
            StatCard(
                label = stringResource(R.string.dashboard_total_inquilinos),
                value = CurrencyFormatter.format(
                    BigDecimal(stats.ingresoMensual),
                    "DOP",
                ),
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun StatCard(
    label: String,
    value: String,
    modifier: Modifier = Modifier,
) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer,
        ),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = value,
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.onPrimaryContainer,
                textAlign = TextAlign.Center,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = label,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onPrimaryContainer.copy(alpha = 0.7f),
                textAlign = TextAlign.Center,
            )
        }
    }
}

@Composable
private fun SectionHeader(
    title: String,
    modifier: Modifier = Modifier,
) {
    Text(
        text = title,
        style = MaterialTheme.typography.titleMedium,
        fontWeight = FontWeight.SemiBold,
        modifier = modifier.padding(top = 8.dp),
    )
}

@Composable
private fun PagoProximoItem(
    pago: PagoProximo,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = pago.propiedadTitulo,
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.Medium,
            )
            Text(
                text = pago.inquilinoNombre,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(
                    text = CurrencyFormatter.format(BigDecimal(pago.monto), pago.moneda),
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    text = pago.fechaVencimiento,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun ContratoCalendarioItem(
    contrato: ContratoCalendario,
    modifier: Modifier = Modifier,
) {
    val indicatorColor = parseColorIndicator(contrato.color)

    Card(modifier = modifier.fillMaxWidth()) {
        Row(
            modifier = Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Spacer(
                modifier = Modifier
                    .size(12.dp)
                    .clip(CircleShape)
                    .background(indicatorColor),
            )
            Spacer(modifier = Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = contrato.propiedadTitulo,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Medium,
                )
                Text(
                    text = contrato.inquilinoNombre,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    text = stringResource(
                        R.string.contrato_dias_restantes,
                        contrato.diasRestantes.toInt(),
                    ),
                    style = MaterialTheme.typography.bodySmall,
                    color = indicatorColor,
                )
            }
            Text(
                text = contrato.fechaFin,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

private fun parseColorIndicator(color: String): Color {
    return when (color.lowercase()) {
        "red", "rojo" -> Color(0xFFD32F2F)
        "yellow", "amarillo" -> Color(0xFFF9A825)
        "green", "verde" -> Color(0xFF388E3C)
        else -> {
            runCatching { Color(android.graphics.Color.parseColor(color)) }
                .getOrDefault(Color(0xFF757575))
        }
    }
}

@Composable
private fun OcupacionTendenciaItem(
    tendencia: OcupacionTendencia,
    modifier: Modifier = Modifier,
) {
    val monthName = getMonthName(tendencia.mes)

    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "$monthName ${tendencia.anio}",
            style = MaterialTheme.typography.bodyMedium,
        )
        Text(
            text = String.format("%.1f%%", tendencia.tasa),
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.SemiBold,
        )
    }
    HorizontalDivider()
}

private fun getMonthName(mes: Int): String {
    return when (mes) {
        1 -> "Enero"
        2 -> "Febrero"
        3 -> "Marzo"
        4 -> "Abril"
        5 -> "Mayo"
        6 -> "Junio"
        7 -> "Julio"
        8 -> "Agosto"
        9 -> "Septiembre"
        10 -> "Octubre"
        11 -> "Noviembre"
        12 -> "Diciembre"
        else -> "Mes $mes"
    }
}

@Composable
private fun IngresosComparacionCard(
    ingresos: IngresosComparacion,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            ComparisonRow(
                label = stringResource(R.string.dashboard_esperado),
                value = CurrencyFormatter.format(BigDecimal(ingresos.esperado), "DOP"),
            )
            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
            ComparisonRow(
                label = stringResource(R.string.dashboard_cobrado),
                value = CurrencyFormatter.format(BigDecimal(ingresos.cobrado), "DOP"),
            )
            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
            ComparisonRow(
                label = stringResource(R.string.dashboard_diferencia),
                value = CurrencyFormatter.format(BigDecimal(ingresos.diferencia), "DOP"),
                valueColor = if (BigDecimal(ingresos.diferencia) >= BigDecimal.ZERO) {
                    Color(0xFF388E3C)
                } else {
                    Color(0xFFD32F2F)
                },
            )
        }
    }
}

@Composable
private fun GastosComparacionCard(
    gastos: GastosComparacion,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            ComparisonRow(
                label = stringResource(R.string.dashboard_mes_actual),
                value = CurrencyFormatter.format(BigDecimal(gastos.mesActual), "DOP"),
            )
            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
            ComparisonRow(
                label = stringResource(R.string.dashboard_mes_anterior),
                value = CurrencyFormatter.format(BigDecimal(gastos.mesAnterior), "DOP"),
            )
            HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
            val changeColor = if (gastos.porcentajeCambio <= 0) {
                Color(0xFF388E3C)
            } else {
                Color(0xFFD32F2F)
            }
            ComparisonRow(
                label = "% Cambio",
                value = String.format("%+.1f%%", gastos.porcentajeCambio),
                valueColor = changeColor,
            )
        }
    }
}

@Composable
private fun ComparisonRow(
    label: String,
    value: String,
    modifier: Modifier = Modifier,
    valueColor: Color = MaterialTheme.colorScheme.onSurface,
) {
    Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.SemiBold,
            color = valueColor,
        )
    }
}
