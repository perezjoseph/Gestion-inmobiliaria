package com.propmanager.feature.configuracion

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.OfflineIndicator
import com.propmanager.core.ui.components.PropManagerTextField
import com.propmanager.core.ui.components.PropManagerTopAppBar

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ConfiguracionScreen(
    viewModel: ConfiguracionViewModel,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val isOnline by viewModel.isOnline.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(uiState.saveSuccess) {
        if (uiState.saveSuccess)
            snackbarHostState.showSnackbar("Configuración actualizada correctamente")
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.configuracion_title),
                onNavigateBack = onNavigateBack,
                scrollBehavior = scrollBehavior,
            )
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
            OfflineIndicator(isOffline = !isOnline)

            when {
                uiState.isLoading -> LoadingScreen()
                uiState.errorMessage != null && uiState.tasa.isEmpty() ->
                    ErrorScreen(
                        message = uiState.errorMessage.orEmpty(),
                        onRetry = viewModel::loadMoneda,
                    )
                else ->
                    ConfiguracionForm(
                        uiState = uiState,
                        onTasaChange = viewModel::onTasaChange,
                        onSave = viewModel::saveMoneda,
                    )
            }
        }
    }
}

@Composable
private fun ConfiguracionForm(
    uiState: ConfiguracionUiState,
    onTasaChange: (String) -> Unit,
    onSave: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.fillMaxSize().padding(16.dp)) {
        Text(
            text = stringResource(R.string.configuracion_moneda),
            style = MaterialTheme.typography.titleMedium,
        )
        Spacer(modifier = Modifier.height(12.dp))
        PropManagerTextField(
            value = uiState.tasa,
            onValueChange = onTasaChange,
            label = "Tasa de cambio (USD → DOP)",
            modifier = Modifier.fillMaxWidth(),
        )
        if (uiState.actualizado.isNotEmpty()) {
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = "Última actualización: ${uiState.actualizado}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        if (uiState.errorMessage != null) {
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = uiState.errorMessage,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
            )
        }
        Spacer(modifier = Modifier.height(16.dp))
        Button(onClick = onSave, enabled = !uiState.isSaving, modifier = Modifier.fillMaxWidth()) {
            Text(stringResource(R.string.save))
        }
    }
}
