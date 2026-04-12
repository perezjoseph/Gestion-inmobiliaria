package com.propmanager.feature.mantenimiento

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.PropManagerTextField
import com.propmanager.core.ui.components.PropManagerTopAppBar

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SolicitudFormScreen(
    viewModel: MantenimientoViewModel,
    isEditing: Boolean,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val formState by viewModel.formState.collectAsStateWithLifecycle()
    val propiedades by viewModel.propiedades.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()
    val title = if (isEditing) stringResource(R.string.solicitud_edit) else stringResource(R.string.solicitud_create)

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
                Modifier
                    .fillMaxSize()
                    .padding(paddingValues)
                    .padding(horizontal = 16.dp)
                    .verticalScroll(rememberScrollState()),
        ) {
            Spacer(Modifier.height(8.dp))
            formState.errors["general"]?.let {
                Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                Spacer(Modifier.height(8.dp))
            }

            var propExpanded by remember { mutableStateOf(false) }
            val selectedProp = propiedades.find { it.id == formState.propiedadId }
            ExposedDropdownMenuBox(expanded = propExpanded, onExpandedChange = { propExpanded = it }) {
                OutlinedTextField(
                    value = selectedProp?.titulo ?: "",
                    onValueChange = {},
                    readOnly = true,
                    label = { Text(stringResource(R.string.solicitud_propiedad)) },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = propExpanded) },
                    isError = formState.errors.containsKey("propiedadId"),
                    modifier = Modifier.fillMaxWidth().menuAnchor(MenuAnchorType.PrimaryNotEditable),
                )
                ExposedDropdownMenu(expanded = propExpanded, onDismissRequest = { propExpanded = false }) {
                    propiedades.forEach { p ->
                        DropdownMenuItem(text = { Text(p.titulo) }, onClick = {
                            viewModel.onFieldChange("propiedadId", p.id)
                            propExpanded =
                                false
                        })
                    }
                }
            }
            formState.errors["propiedadId"]?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(start = 16.dp, top = 4.dp),
                )
            }

            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.titulo, onValueChange = {
                viewModel.onFieldChange("titulo", it)
            }, label = stringResource(R.string.solicitud_titulo), error = formState.errors["titulo"], modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.descripcion, onValueChange = {
                viewModel.onFieldChange("descripcion", it)
            }, label = stringResource(R.string.solicitud_descripcion), singleLine = false, maxLines = 4, modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))

            var prioExpanded by remember { mutableStateOf(false) }
            val prioridades = listOf("baja", "media", "alta", "urgente")
            ExposedDropdownMenuBox(expanded = prioExpanded, onExpandedChange = { prioExpanded = it }) {
                OutlinedTextField(
                    value = formState.prioridad,
                    onValueChange = {},
                    readOnly = true,
                    label = { Text(stringResource(R.string.solicitud_prioridad)) },
                    trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = prioExpanded) },
                    modifier = Modifier.fillMaxWidth().menuAnchor(MenuAnchorType.PrimaryNotEditable),
                )
                ExposedDropdownMenu(expanded = prioExpanded, onDismissRequest = { prioExpanded = false }) {
                    prioridades.forEach { p ->
                        DropdownMenuItem(text = { Text(p) }, onClick = {
                            viewModel.onFieldChange("prioridad", p)
                            prioExpanded =
                                false
                        })
                    }
                }
            }

            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.nombreProveedor, onValueChange = {
                viewModel.onFieldChange("nombreProveedor", it)
            }, label = stringResource(R.string.solicitud_proveedor_nombre), modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.telefonoProveedor, onValueChange = {
                viewModel.onFieldChange("telefonoProveedor", it)
            }, label = stringResource(R.string.solicitud_proveedor_telefono), modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.emailProveedor, onValueChange = {
                viewModel.onFieldChange("emailProveedor", it)
            }, label = stringResource(R.string.solicitud_proveedor_email), modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.costoMonto, onValueChange = {
                viewModel.onFieldChange("costoMonto", it)
            }, label = stringResource(R.string.solicitud_costo), modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(value = formState.costoMoneda, onValueChange = {
                viewModel.onFieldChange("costoMoneda", it)
            }, label = stringResource(R.string.pago_moneda), modifier = Modifier.fillMaxWidth())
            Spacer(Modifier.height(16.dp))
            Button(
                onClick = { viewModel.save(onSuccess = onNavigateBack) },
                enabled = !formState.isSubmitting,
                modifier = Modifier.fillMaxWidth(),
            ) {
                if (formState.isSubmitting) CircularProgressIndicator() else Text(stringResource(R.string.save))
            }
            Spacer(Modifier.height(16.dp))
        }
    }
}
