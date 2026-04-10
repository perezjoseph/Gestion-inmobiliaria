package com.propmanager.navigation

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.PropManagerTopAppBar

private data class MenuItem(val labelResId: Int, val route: String)

private val menuItems =
    listOf(
        MenuItem(R.string.nav_pagos, Routes.PAGOS),
        MenuItem(R.string.nav_gastos, Routes.GASTOS),
        MenuItem(R.string.nav_mantenimiento, Routes.MANTENIMIENTO),
        MenuItem(R.string.nav_reportes, Routes.REPORTES),
        MenuItem(R.string.nav_documentos, "documentos/propiedad/all"),
        MenuItem(R.string.nav_notificaciones, Routes.NOTIFICACIONES),
        MenuItem(R.string.nav_auditoria, Routes.AUDITORIA),
        MenuItem(R.string.nav_perfil, Routes.PERFIL),
        MenuItem(R.string.nav_configuracion, Routes.CONFIGURACION),
        MenuItem(R.string.nav_importacion, Routes.IMPORTACION),
    )

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MasMenuScreen(
    onNavigate: (String) -> Unit,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.nav_mas),
                onNavigateBack = onNavigateBack,
                scrollBehavior = scrollBehavior,
            )
        },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(
            modifier =
                Modifier.fillMaxSize().padding(paddingValues).verticalScroll(rememberScrollState())
        ) {
            menuItems.forEach { item ->
                ListItem(
                    headlineContent = {
                        Text(
                            text = stringResource(item.labelResId),
                            style = MaterialTheme.typography.bodyLarge,
                        )
                    },
                    modifier = Modifier.fillMaxWidth().clickable { onNavigate(item.route) },
                )
                HorizontalDivider()
            }
        }
    }
}
