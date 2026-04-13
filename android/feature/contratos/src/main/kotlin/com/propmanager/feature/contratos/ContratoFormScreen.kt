package com.propmanager.feature.contratos

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
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
import com.propmanager.core.ui.components.DatePickerField
import com.propmanager.core.ui.components.PropManagerTextField
import com.propmanager.core.ui.components.PropManagerTopAppBar

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ContratoFormScreen(
    viewModel: ContratosViewModel,
    isEditing: Boolean,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val formState by viewModel.formState.collectAsStateWithLifecycle()
    val propiedades by viewModel.propiedades.collectAsStateWithLifecycle()
    val inquilinos by viewModel.inquilinos.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()
    val title =
        if (isEditing) stringResource(R.string.contrato_edit)
        else stringResource(R.string.contrato_create)

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

            var propExpanded by remember { mutableStateOf(false) }
            val selectedProp = propiedades.find { it.id == formState.propiedadId }
            ExposedDropdownMenuBox(
                expanded = propExpanded,
                onExpandedChange = { propExpanded = it },
            ) {
                OutlinedTextField(
                    value = selectedProp?.titulo ?: "",
                    onValueChange = {},
                    readOnly = true,
                    label = { Text(stringResource(R.string.contrato_propiedad)) },
                    trailingIcon = {
                        ExposedDropdownMenuDefaults.TrailingIcon(expanded = propExpanded)
                    },
                    isError = formState.errors.containsKey("propiedadId"),
                    modifier = Modifier.fillMaxWidth().menuAnchor(MenuAnchorType.PrimaryNotEditable),
                )
                ExposedDropdownMenu(
                    expanded = propExpanded,
                    onDismissRequest = { propExpanded = false },
                ) {
                    propiedades.forEach { p ->
                        DropdownMenuItem(
                            text = { Text(p.titulo) },
                            onClick = {
                                viewModel.onFormFieldChange("propiedadId", p.id)
                                propExpanded = false
                            },
                        )
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

            var inqExpanded by remember { mutableStateOf(false) }
            val selectedInq = inquilinos.find { it.id == formState.inquilinoId }
            ExposedDropdownMenuBox(
                expanded = inqExpanded,
                onExpandedChange = { inqExpanded = it },
            ) {
                OutlinedTextField(
                    value = selectedInq?.let { "${it.nombre} ${it.apellido}" } ?: "",
                    onValueChange = {},
                    readOnly = true,
                    label = { Text(stringResource(R.string.contrato_inquilino)) },
                    trailingIcon = {
                        ExposedDropdownMenuDefaults.TrailingIcon(expanded = inqExpanded)
                    },
                    isError = formState.errors.containsKey("inquilinoId"),
                    modifier = Modifier.fillMaxWidth().menuAnchor(MenuAnchorType.PrimaryNotEditable),
                )
                ExposedDropdownMenu(
                    expanded = inqExpanded,
                    onDismissRequest = { inqExpanded = false },
                ) {
                    inquilinos.forEach { i ->
                        DropdownMenuItem(
                            text = { Text("${i.nombre} ${i.apellido}") },
                            onClick = {
                                viewModel.onFormFieldChange("inquilinoId", i.id)
                                inqExpanded = false
                            },
                        )
                    }
                }
            }
            formState.errors["inquilinoId"]?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(start = 16.dp, top = 4.dp),
                )
            }

            Spacer(Modifier.height(8.dp))
            DatePickerField(
                value = formState.fechaInicio,
                onValueChange = viewModel::onFechaInicioChange,
                label = stringResource(R.string.contrato_fecha_inicio),
                error = formState.errors["fechaInicio"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            DatePickerField(
                value = formState.fechaFin,
                onValueChange = viewModel::onFechaFinChange,
                label = stringResource(R.string.contrato_fecha_fin),
                error = formState.errors["fechaFin"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.montoMensual,
                onValueChange = { viewModel.onFormFieldChange("montoMensual", it) },
                label = stringResource(R.string.contrato_monto_mensual),
                error = formState.errors["montoMensual"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.deposito,
                onValueChange = { viewModel.onFormFieldChange("deposito", it) },
                label = stringResource(R.string.contrato_deposito),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.moneda,
                onValueChange = { viewModel.onFormFieldChange("moneda", it) },
                label = stringResource(R.string.contrato_moneda),
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
