package com.propmanager.feature.inquilinos

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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.model.Inquilino
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ConfirmDeleteDialog
import com.propmanager.core.ui.components.EmptyStateScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.OfflineIndicator
import com.propmanager.core.ui.components.PropManagerTextField
import com.propmanager.core.ui.components.PropManagerTopAppBar
import com.propmanager.core.ui.components.SyncStatusBadge

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InquilinosListScreen(
    viewModel: InquilinosViewModel,
    onNavigateToCreate: () -> Unit,
    onNavigateToEdit: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.inquilinos.collectAsStateWithLifecycle()
    val searchQuery by viewModel.searchQuery.collectAsStateWithLifecycle()
    val isOnline by viewModel.isOnline.collectAsStateWithLifecycle()
    val deleteTarget by viewModel.deleteTarget.collectAsStateWithLifecycle()
    val successMessage by viewModel.successMessage.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    LaunchedEffect(successMessage) {
        successMessage?.let {
            snackbarHostState.showSnackbar(it)
            viewModel.clearSuccessMessage()
        }
    }

    deleteTarget?.let { inquilino ->
        ConfirmDeleteDialog(
            itemName = "${inquilino.nombre} ${inquilino.apellido}",
            onConfirm = viewModel::confirmDelete,
            onDismiss = viewModel::dismissDelete,
        )
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.inquilinos_title),
                scrollBehavior = scrollBehavior,
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = {
                    viewModel.initCreateForm()
                    onNavigateToCreate()
                }
            ) {
                Icon(
                    Icons.Filled.Add,
                    contentDescription = stringResource(R.string.inquilino_create),
                )
            }
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.padding(paddingValues)) {
            OfflineIndicator(isOffline = !isOnline)
            PropManagerTextField(
                value = searchQuery,
                onValueChange = viewModel::onSearchChange,
                label = stringResource(R.string.search),
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
                leadingIcon = { Icon(Icons.Filled.Search, contentDescription = null) },
            )
            when (val state = uiState) {
                is InquilinosUiState.Loading -> LoadingScreen()
                is InquilinosUiState.Success -> {
                    if (state.inquilinos.isEmpty()) {
                        EmptyStateScreen(message = stringResource(R.string.inquilino_empty))
                    } else {
                        LazyColumn(
                            modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            items(state.inquilinos, key = { it.id }) { inquilino ->
                                InquilinoListItem(
                                    inquilino = inquilino,
                                    onClick = { onNavigateToEdit(inquilino.id) },
                                    onDelete = { viewModel.requestDelete(inquilino) },
                                )
                            }
                            item { Spacer(Modifier.height(80.dp)) }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun InquilinoListItem(
    inquilino: Inquilino,
    onClick: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth().clickable(onClick = onClick)) {
        Row(modifier = Modifier.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        "${inquilino.nombre} ${inquilino.apellido}",
                        style = MaterialTheme.typography.bodyLarge,
                        fontWeight = FontWeight.Medium,
                    )
                    Spacer(Modifier.width(4.dp))
                    SyncStatusBadge(isPendingSync = inquilino.isPendingSync)
                }
                Text(
                    inquilino.cedula,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                inquilino.telefono?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            IconButton(onClick = onDelete) {
                Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.delete))
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InquilinoFormScreen(
    viewModel: InquilinosViewModel,
    isEditing: Boolean,
    onNavigateBack: () -> Unit,
    onScanCedula: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val formState by viewModel.formState.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()
    val title =
        if (isEditing) stringResource(R.string.inquilino_edit)
        else stringResource(R.string.inquilino_create)

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = title,
                scrollBehavior = scrollBehavior,
                onNavigateBack = onNavigateBack,
            )
        },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(
            modifier =
                Modifier.fillMaxSize()
                    .padding(paddingValues)
                    .padding(horizontal = 16.dp)
                    .verticalScroll(rememberScrollState())
        ) {
            Spacer(Modifier.height(8.dp))
            formState.errors["general"]?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                )
                Spacer(Modifier.height(8.dp))
            }
            PropManagerTextField(
                value = formState.nombre,
                onValueChange = { viewModel.onFieldChange("nombre", it) },
                label = stringResource(R.string.inquilino_nombre),
                error = formState.errors["nombre"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.apellido,
                onValueChange = { viewModel.onFieldChange("apellido", it) },
                label = stringResource(R.string.inquilino_apellido),
                error = formState.errors["apellido"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.cedula,
                onValueChange = { viewModel.onFieldChange("cedula", it) },
                label = stringResource(R.string.inquilino_cedula),
                error = formState.errors["cedula"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            OutlinedButton(onClick = onScanCedula, modifier = Modifier.fillMaxWidth()) {
                Text(stringResource(R.string.inquilino_scan_cedula))
            }
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.email,
                onValueChange = { viewModel.onFieldChange("email", it) },
                label = stringResource(R.string.inquilino_email),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.telefono,
                onValueChange = { viewModel.onFieldChange("telefono", it) },
                label = stringResource(R.string.inquilino_telefono),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.contactoEmergencia,
                onValueChange = { viewModel.onFieldChange("contactoEmergencia", it) },
                label = stringResource(R.string.inquilino_contacto_emergencia),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.notas,
                onValueChange = { viewModel.onFieldChange("notas", it) },
                label = stringResource(R.string.inquilino_notas),
                singleLine = false,
                maxLines = 4,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(16.dp))
            Button(
                onClick = { viewModel.save(onSuccess = onNavigateBack) },
                enabled = !formState.isSubmitting,
                modifier = Modifier.fillMaxWidth(),
            ) {
                if (formState.isSubmitting) CircularProgressIndicator()
                else Text(stringResource(R.string.save))
            }
            Spacer(Modifier.height(16.dp))
        }
    }
}
