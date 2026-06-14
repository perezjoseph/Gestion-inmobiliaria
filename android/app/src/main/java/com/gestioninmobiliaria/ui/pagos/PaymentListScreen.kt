package com.gestioninmobiliaria.ui.pagos

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import com.gestioninmobiliaria.data.model.EstadoPago
import com.gestioninmobiliaria.data.model.Pago
import java.text.NumberFormat
import java.util.Locale

private val PendienteColor = Color(0xFFF59E0B) // amber
private val PagadoColor = Color(0xFF10B981)    // green
private val AtrasadoColor = Color(0xFFEF4444)  // red

private fun estadoColor(estado: EstadoPago): Color = when (estado) {
    EstadoPago.pendiente -> PendienteColor
    EstadoPago.pagado -> PagadoColor
    EstadoPago.atrasado -> AtrasadoColor
}

private fun estadoLabel(estado: EstadoPago): String = when (estado) {
    EstadoPago.pendiente -> "Pendiente"
    EstadoPago.pagado -> "Pagado"
    EstadoPago.atrasado -> "Atrasado"
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaymentListScreen(
    viewModel: PaymentListViewModel = hiltViewModel(),
) {
    val state by viewModel.uiState.collectAsState()

    Scaffold(
        topBar = { TopAppBar(title = { Text("Pagos") }) },
    ) { padding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
        ) {
            when {
                state.isLoading -> CircularProgressIndicator(Modifier.align(Alignment.Center))
                state.error != null -> {
                    Column(
                        Modifier.align(Alignment.Center),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Text(state.error!!, color = MaterialTheme.colorScheme.error)
                        Spacer(Modifier.height(8.dp))
                        Button(onClick = { viewModel.load() }) { Text("Reintentar") }
                    }
                }
                else -> PaymentGroupedList(state.grouped)
            }
        }
    }
}

@Composable
private fun PaymentGroupedList(grouped: Map<EstadoPago, List<Pago>>) {
    LazyColumn(
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        grouped.forEach { (estado, pagos) ->
            item(key = "header_$estado") {
                Text(
                    text = "${estadoLabel(estado)} (${pagos.size})",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(vertical = 8.dp),
                )
            }
            items(pagos, key = { it.id }) { pago ->
                PaymentCard(pago)
            }
        }
    }
}

@Composable
private fun PaymentCard(pago: Pago) {
    val formatter = NumberFormat.getNumberInstance(Locale("es", "DO")).apply {
        minimumFractionDigits = 2
        maximumFractionDigits = 2
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier = Modifier
                .padding(16.dp)
                .fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(
                    text = "${pago.moneda} ${formatter.format(pago.monto)}",
                    style = MaterialTheme.typography.bodyLarge,
                )
                Text(
                    text = "Vence: ${pago.fecha_vencimiento}",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            StatusChip(pago.estado)
        }
    }
}

@Composable
private fun StatusChip(estado: EstadoPago) {
    val color = estadoColor(estado)
    Surface(
        color = color.copy(alpha = 0.12f),
        shape = MaterialTheme.shapes.small,
    ) {
        Text(
            text = estadoLabel(estado),
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp),
            style = MaterialTheme.typography.labelMedium,
            color = color,
        )
    }
}
