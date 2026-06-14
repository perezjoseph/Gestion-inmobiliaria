package com.gestioninmobiliaria.ui.pagos

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel

private val MONEDAS = listOf("DOP", "USD")
private val METODOS_PAGO = listOf("efectivo", "transferencia", "cheque", "tarjeta")

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PaymentFormScreen(
    onBack: () -> Unit,
    viewModel: PaymentFormViewModel = hiltViewModel(),
) {
    val contratoId by viewModel.contratoId.collectAsState()
    val monto by viewModel.monto.collectAsState()
    val moneda by viewModel.moneda.collectAsState()
    val metodoPago by viewModel.metodoPago.collectAsState()
    val fechaPago by viewModel.fechaPago.collectAsState()
    val isSaving by viewModel.isSaving.collectAsState()
    val saveError by viewModel.saveError.collectAsState()
    val saved by viewModel.saved.collectAsState()

    LaunchedEffect(saved) {
        if (saved) onBack()
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Registrar Pago") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Volver")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            OutlinedTextField(
                value = contratoId,
                onValueChange = viewModel::updateContratoId,
                label = { Text("ID de Contrato") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            OutlinedTextField(
                value = monto,
                onValueChange = viewModel::updateMonto,
                label = { Text("Monto") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            MonedaSelector(selected = moneda, onSelect = viewModel::updateMoneda)

            MetodoPagoSelector(selected = metodoPago, onSelect = viewModel::updateMetodoPago)

            OutlinedTextField(
                value = fechaPago,
                onValueChange = viewModel::updateFechaPago,
                label = { Text("Fecha de Pago (DD/MM/YYYY)") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            if (saveError != null) {
                Text(saveError!!, color = MaterialTheme.colorScheme.error)
            }

            Spacer(Modifier.height(8.dp))

            Button(
                onClick = viewModel::submit,
                modifier = Modifier.fillMaxWidth(),
                enabled = !isSaving,
            ) {
                if (isSaving) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(18.dp),
                        strokeWidth = 2.dp,
                    )
                    Spacer(Modifier.width(8.dp))
                }
                Text("Guardar Pago")
            }

            OutlinedButton(
                onClick = viewModel::clearDraft,
                modifier = Modifier.fillMaxWidth(),
                enabled = !isSaving,
            ) {
                Text("Limpiar Borrador")
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MonedaSelector(selected: String, onSelect: (String) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
        OutlinedTextField(
            value = selected,
            onValueChange = {},
            readOnly = true,
            label = { Text("Moneda") },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) },
            modifier = Modifier.fillMaxWidth().menuAnchor(),
        )
        ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            MONEDAS.forEach { m ->
                DropdownMenuItem(text = { Text(m) }, onClick = { onSelect(m); expanded = false })
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun MetodoPagoSelector(selected: String, onSelect: (String) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }) {
        OutlinedTextField(
            value = selected.replaceFirstChar { it.uppercase() },
            onValueChange = {},
            readOnly = true,
            label = { Text("Método de Pago") },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) },
            modifier = Modifier.fillMaxWidth().menuAnchor(),
        )
        ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            METODOS_PAGO.forEach { m ->
                DropdownMenuItem(
                    text = { Text(m.replaceFirstChar { it.uppercase() }) },
                    onClick = { onSelect(m); expanded = false },
                )
            }
        }
    }
}
