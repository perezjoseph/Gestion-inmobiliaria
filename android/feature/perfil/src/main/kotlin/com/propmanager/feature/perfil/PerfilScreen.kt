package com.propmanager.feature.perfil

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.material3.OutlinedTextField
import androidx.compose.ui.text.input.PasswordVisualTransformation
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
fun PerfilScreen(
    viewModel: PerfilViewModel,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val isOnline by viewModel.isOnline.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()
    val snackbarHostState = remember { SnackbarHostState() }

    LaunchedEffect(uiState.saveSuccess) {
        if (uiState.saveSuccess) snackbarHostState.showSnackbar("Perfil actualizado correctamente")
    }
    LaunchedEffect(uiState.passwordChanged) {
        if (uiState.passwordChanged) snackbarHostState.showSnackbar("Contraseña actualizada correctamente")
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.perfil_title),
                onNavigateBack = onNavigateBack,
                scrollBehavior = scrollBehavior,
            )
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
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
                uiState.errorMessage != null && uiState.nombre.isEmpty() -> ErrorScreen(
                    message = uiState.errorMessage!!,
                    onRetry = viewModel::loadPerfil,
                )
                else -> PerfilContent(
                    uiState = uiState,
                    onNombreChange = viewModel::onNombreChange,
                    onSave = viewModel::savePerfil,
                    onTogglePasswordForm = viewModel::togglePasswordForm,
                    onPasswordActualChange = viewModel::onPasswordActualChange,
                    onPasswordNuevaChange = viewModel::onPasswordNuevaChange,
                    onChangePassword = viewModel::changePassword,
                )
            }
        }
    }
}

@Composable
private fun PerfilContent(
    uiState: PerfilUiState,
    onNombreChange: (String) -> Unit,
    onSave: () -> Unit,
    onTogglePasswordForm: () -> Unit,
    onPasswordActualChange: (String) -> Unit,
    onPasswordNuevaChange: (String) -> Unit,
    onChangePassword: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
    ) {
        PropManagerTextField(
            value = uiState.nombre,
            onValueChange = onNombreChange,
            label = stringResource(R.string.perfil_nombre),
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(modifier = Modifier.height(8.dp))
        PropManagerTextField(
            value = uiState.email,
            onValueChange = {},
            label = stringResource(R.string.perfil_email),
            enabled = false,
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(modifier = Modifier.height(8.dp))
        PropManagerTextField(
            value = uiState.rol,
            onValueChange = {},
            label = stringResource(R.string.perfil_rol),
            enabled = false,
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(modifier = Modifier.height(16.dp))

        if (uiState.errorMessage != null) {
            Text(
                text = uiState.errorMessage,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.padding(bottom = 8.dp),
            )
        }

        Button(
            onClick = onSave,
            enabled = !uiState.isSaving,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(stringResource(R.string.save))
        }

        Spacer(modifier = Modifier.height(24.dp))

        OutlinedButton(
            onClick = onTogglePasswordForm,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(stringResource(R.string.perfil_cambiar_password))
        }

        if (uiState.showPasswordForm) {
            Spacer(modifier = Modifier.height(12.dp))
            OutlinedTextField(
                value = uiState.passwordActual,
                onValueChange = onPasswordActualChange,
                label = { Text(stringResource(R.string.perfil_password_actual)) },
                visualTransformation = PasswordVisualTransformation(),
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(modifier = Modifier.height(8.dp))
            OutlinedTextField(
                value = uiState.passwordNueva,
                onValueChange = onPasswordNuevaChange,
                label = { Text(stringResource(R.string.perfil_password_nueva)) },
                visualTransformation = PasswordVisualTransformation(),
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(modifier = Modifier.height(12.dp))
            Button(
                onClick = onChangePassword,
                enabled = !uiState.isChangingPassword,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(stringResource(R.string.perfil_cambiar_password))
            }
        }
    }
}
