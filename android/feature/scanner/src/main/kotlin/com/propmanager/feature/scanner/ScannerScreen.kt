package com.propmanager.feature.scanner

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.PropManagerTopAppBar

enum class ScannerMode { CEDULA, RECEIPT }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ScannerScreen(
    viewModel: ScannerViewModel,
    mode: ScannerMode,
    onNavigateBack: () -> Unit,
    onConfirmCedula: (CedulaOcrResult) -> Unit,
    onConfirmReceipt: (ReceiptOcrResult) -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.scanner_title),
                onNavigateBack = onNavigateBack,
                scrollBehavior = scrollBehavior,
            )
        },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(paddingValues)
                    .verticalScroll(rememberScrollState()),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            CameraPreviewPlaceholder(
                onCapture = { viewModel.onCaptureRequested(mode) },
                isProcessing = uiState.isProcessing,
            )

            if (uiState.isProcessing) {
                Spacer(modifier = Modifier.height(24.dp))
                CircularProgressIndicator()
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = stringResource(R.string.loading),
                    style = MaterialTheme.typography.bodyMedium,
                )
            }

            uiState.errorMessage?.let { error ->
                Spacer(modifier = Modifier.height(16.dp))
                Text(
                    text = error,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.padding(horizontal = 16.dp),
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = stringResource(R.string.scanner_retry),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = 16.dp),
                )
            }

            uiState.cedulaResult?.let { result ->
                Spacer(modifier = Modifier.height(16.dp))
                CedulaResultPreview(
                    result = result,
                    onConfirm = { onConfirmCedula(result) },
                    onRetake = viewModel::reset,
                )
            }

            uiState.receiptResult?.let { result ->
                Spacer(modifier = Modifier.height(16.dp))
                ReceiptResultPreview(
                    result = result,
                    onConfirm = { onConfirmReceipt(result) },
                    onRetake = viewModel::reset,
                )
            }
        }
    }
}

@Composable
private fun CameraPreviewPlaceholder(
    onCapture: () -> Unit,
    isProcessing: Boolean,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier =
            modifier
                .fillMaxWidth()
                .aspectRatio(4f / 3f)
                .background(MaterialTheme.colorScheme.surfaceVariant),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(
                text = "Vista previa de cámara",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
            Spacer(modifier = Modifier.height(16.dp))
            Button(
                onClick = onCapture,
                enabled = !isProcessing,
            ) {
                Text("Capturar")
            }
        }
    }
}

@Composable
private fun CedulaResultPreview(
    result: CedulaOcrResult,
    onConfirm: () -> Unit,
    onRetake: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth().padding(16.dp)) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text("Resultado del escaneo", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            Spacer(modifier = Modifier.height(8.dp))
            ResultField("Nombre", result.nombre)
            ResultField("Apellido", result.apellido)
            ResultField("Cédula", result.cedula)
            Text(
                text = "Confianza: ${String.format("%.0f%%", result.confidence * 100)}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(12.dp))
            ConfirmRetakeButtons(onConfirm = onConfirm, onRetake = onRetake)
        }
    }
}

@Composable
private fun ReceiptResultPreview(
    result: ReceiptOcrResult,
    onConfirm: () -> Unit,
    onRetake: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth().padding(16.dp)) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text("Resultado del escaneo", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            Spacer(modifier = Modifier.height(8.dp))
            ResultField("Monto", result.monto?.toPlainString())
            ResultField("Fecha", result.fecha?.toString())
            ResultField("Proveedor", result.proveedor)
            ResultField("No. Factura", result.numeroFactura)
            Text(
                text = "Confianza: ${String.format("%.0f%%", result.confidence * 100)}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(modifier = Modifier.height(12.dp))
            ConfirmRetakeButtons(onConfirm = onConfirm, onRetake = onRetake)
        }
    }
}

@Composable
private fun ResultField(
    label: String,
    value: String?,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.fillMaxWidth().padding(vertical = 2.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(label, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value ?: "—", style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.Medium)
    }
}

@Composable
private fun ConfirmRetakeButtons(
    onConfirm: () -> Unit,
    onRetake: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(modifier = modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(onClick = onRetake, modifier = Modifier.weight(1f)) {
            Text("Reintentar")
        }
        Button(onClick = onConfirm, modifier = Modifier.weight(1f)) {
            Text(stringResource(R.string.confirm))
        }
    }
}
