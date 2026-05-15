package com.propmanager.feature.usuarios

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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import com.propmanager.core.model.dto.UserDto
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.EmptyStateScreen
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.PropManagerTopAppBar
import kotlinx.collections.immutable.ImmutableList

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun UsuariosScreen(
    viewModel: UsuariosViewModel,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val actionError by viewModel.actionError.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    LaunchedEffect(actionError) {
        actionError?.let {
            snackbarHostState.showSnackbar(it)
            viewModel.clearActionError()
        }
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.nav_usuarios),
                scrollBehavior = scrollBehavior,
                onNavigateBack = onNavigateBack,
            )
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.padding(paddingValues)) {
            when (val state = uiState) {
                is UsuariosUiState.Loading -> LoadingScreen()
                is UsuariosUiState.Error -> {
                    ErrorScreen(
                        message = state.message,
                        onRetry = remember { { viewModel.loadUsuarios(1) } },
                    )
                }
                is UsuariosUiState.Success -> {
                    UsuariosContent(
                        users = state.users,
                        page = state.page,
                        totalPages = state.totalPages,
                        onChangeRole = viewModel::changeRole,
                        onToggleActivo = viewModel::toggleActivo,
                        onPageChange = viewModel::loadUsuarios,
                    )
                }
            }
        }
    }
}

@Composable
private fun UsuariosContent(
    users: ImmutableList<UserDto>,
    page: Int,
    totalPages: Int,
    onChangeRole: (String, String) -> Unit,
    onToggleActivo: (String) -> Unit,
    onPageChange: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    if (users.isEmpty()) {
        EmptyStateScreen(message = stringResource(R.string.usuarios_empty))
    } else {
        LazyColumn(
            modifier = modifier.fillMaxSize().padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            item { Spacer(Modifier.height(4.dp)) }
            items(users, key = { it.id }) { user ->
                UsuarioListItem(
                    user = user,
                    onChangeRole = { newRole -> onChangeRole(user.id, newRole) },
                    onToggleActivo = { onToggleActivo(user.id) },
                )
            }
            item {
                PaginationControls(
                    page = page,
                    totalPages = totalPages,
                    onPageChange = onPageChange,
                )
            }
            item { Spacer(Modifier.height(16.dp)) }
        }
    }
}

@Composable
private fun UsuarioListItem(
    user: UserDto,
    onChangeRole: (String) -> Unit,
    onToggleActivo: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var showToggleConfirmation by remember { mutableStateOf(false) }

    if (showToggleConfirmation) {
        ConfirmToggleActivoDialog(
            userName = user.nombre,
            isCurrentlyActive = user.activo,
            onConfirm = {
                showToggleConfirmation = false
                onToggleActivo()
            },
            onDismiss = { showToggleConfirmation = false },
        )
    }

    Card(modifier = modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = user.nombre,
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.Medium,
                    )
                    Text(
                        text = user.email,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(
                        text = stringResource(R.string.usuarios_activo),
                        style = MaterialTheme.typography.labelSmall,
                    )
                    Switch(
                        checked = user.activo,
                        onCheckedChange = { showToggleConfirmation = true },
                    )
                }
            }
            Spacer(Modifier.height(8.dp))
            RoleDropdown(
                currentRole = user.rol,
                onRoleSelected = onChangeRole,
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun RoleDropdown(
    currentRole: String,
    onRoleSelected: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }
    val roles = remember { listOf("admin", "gerente", "visualizador") }

    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it },
        modifier = modifier,
    ) {
        OutlinedTextField(
            value = currentRole,
            onValueChange = {},
            readOnly = true,
            label = { Text(stringResource(R.string.usuarios_rol)) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier = Modifier.fillMaxWidth().menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
        ) {
            roles.forEach { role ->
                DropdownMenuItem(
                    text = { Text(role) },
                    onClick = {
                        expanded = false
                        if (role != currentRole) {
                            onRoleSelected(role)
                        }
                    },
                )
            }
        }
    }
}

@Composable
private fun PaginationControls(
    page: Int,
    totalPages: Int,
    onPageChange: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.fillMaxWidth().padding(vertical = 8.dp),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        IconButton(
            onClick = { onPageChange(page - 1) },
            enabled = page > 1,
        ) {
            Icon(
                Icons.AutoMirrored.Filled.ArrowBack,
                contentDescription = stringResource(R.string.usuarios_pagina_anterior),
            )
        }
        Spacer(Modifier.width(16.dp))
        Text(
            text = stringResource(R.string.usuarios_pagina_info, page, totalPages),
            style = MaterialTheme.typography.bodyMedium,
        )
        Spacer(Modifier.width(16.dp))
        IconButton(
            onClick = { onPageChange(page + 1) },
            enabled = page < totalPages,
        ) {
            Icon(
                Icons.AutoMirrored.Filled.ArrowForward,
                contentDescription = stringResource(R.string.usuarios_pagina_siguiente),
            )
        }
    }
}

@Composable
private fun ConfirmToggleActivoDialog(
    userName: String,
    isCurrentlyActive: Boolean,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val message = if (isCurrentlyActive) {
        stringResource(R.string.usuarios_confirm_deactivate, userName)
    } else {
        stringResource(R.string.usuarios_confirm_activate, userName)
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        modifier = modifier,
        title = { Text(text = stringResource(R.string.usuarios_confirm_toggle_title)) },
        text = { Text(text = message) },
        confirmButton = {
            TextButton(onClick = onConfirm) {
                Text(text = stringResource(R.string.confirm))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(text = stringResource(R.string.cancel))
            }
        },
    )
}
