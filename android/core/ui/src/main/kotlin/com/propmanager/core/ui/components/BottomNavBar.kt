package com.propmanager.core.ui.components

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import com.propmanager.core.ui.R

enum class BottomNavItem(
    val icon: ImageVector,
    val labelResId: Int,
    val route: String,
) {
    Dashboard(Icons.Filled.Home, R.string.nav_dashboard, "dashboard"),
    Propiedades(Icons.Filled.Home, R.string.nav_propiedades, "propiedades"),
    Inquilinos(Icons.Filled.Person, R.string.nav_inquilinos, "inquilinos"),
    Contratos(Icons.Filled.Home, R.string.nav_contratos, "contratos"),
    Mas(Icons.Filled.Menu, R.string.nav_mas, "mas"),
}

@Composable
fun PropManagerBottomNavBar(
    currentRoute: String?,
    onNavigate: (BottomNavItem) -> Unit,
    modifier: Modifier = Modifier,
) {
    NavigationBar(modifier = modifier) {
        BottomNavItem.entries.forEach { item ->
            NavigationBarItem(
                icon = {
                    Icon(
                        imageVector = item.icon,
                        contentDescription = stringResource(item.labelResId),
                    )
                },
                label = { Text(text = stringResource(item.labelResId)) },
                selected = currentRoute == item.route,
                onClick = { onNavigate(item) },
            )
        }
    }
}
