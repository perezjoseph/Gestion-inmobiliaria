package com.propmanager.feature.propiedades

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
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
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
fun PropiedadFormScreen(
    viewModel: PropiedadesViewModel,
    isEditing: Boolean,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val formState by viewModel.formState.collectAsStateWithLifecycle()
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()
    val title = if (isEditing) stringResource(R.string.propiedad_edit) else stringResource(R.string.propiedad_create)

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

            formState.errors["general"]?.let { error ->
                Text(error, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                Spacer(Modifier.height(8.dp))
            }

            PropManagerTextField(
                value = formState.titulo,
                onValueChange = { viewModel.onFieldChange("titulo", it) },
                label = stringResource(R.string.propiedad_titulo),
                error = formState.errors["titulo"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.direccion,
                onValueChange = { viewModel.onFieldChange("direccion", it) },
                label = stringResource(R.string.propiedad_direccion),
                error = formState.errors["direccion"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.ciudad,
                onValueChange = { viewModel.onFieldChange("ciudad", it) },
                label = stringResource(R.string.propiedad_ciudad),
                error = formState.errors["ciudad"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.provincia,
                onValueChange = { viewModel.onFieldChange("provincia", it) },
                label = stringResource(R.string.propiedad_provincia),
                error = formState.errors["provincia"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.tipoPropiedad,
                onValueChange = { viewModel.onFieldChange("tipoPropiedad", it) },
                label = stringResource(R.string.propiedad_tipo),
                error = formState.errors["tipoPropiedad"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.precio,
                onValueChange = { viewModel.onFieldChange("precio", it) },
                label = stringResource(R.string.propiedad_precio),
                error = formState.errors["precio"],
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.moneda,
                onValueChange = { viewModel.onFieldChange("moneda", it) },
                label = stringResource(R.string.propiedad_moneda),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.estado,
                onValueChange = { viewModel.onFieldChange("estado", it) },
                label = stringResource(R.string.propiedad_estado),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.habitaciones,
                onValueChange = { viewModel.onFieldChange("habitaciones", it) },
                label = stringResource(R.string.propiedad_habitaciones),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.banos,
                onValueChange = { viewModel.onFieldChange("banos", it) },
                label = stringResource(R.string.propiedad_banos),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.areaM2,
                onValueChange = { viewModel.onFieldChange("areaM2", it) },
                label = stringResource(R.string.propiedad_area),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(8.dp))
            PropManagerTextField(
                value = formState.descripcion,
                onValueChange = { viewModel.onFieldChange("descripcion", it) },
                label = stringResource(R.string.propiedad_descripcion),
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
                if (formState.isSubmitting) {
                    CircularProgressIndicator(modifier = Modifier)
                } else {
                    Text(stringResource(R.string.save))
                }
            }
            Spacer(Modifier.height(16.dp))
        }
    }
}
