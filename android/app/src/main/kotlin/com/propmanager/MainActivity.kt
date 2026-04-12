package com.propmanager

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.propmanager.core.data.repository.NotificacionesRepository
import com.propmanager.core.network.NetworkMonitor
import com.propmanager.core.ui.components.PropManagerBottomNavBar
import com.propmanager.core.ui.theme.PropManagerTheme
import com.propmanager.feature.auth.AuthState
import com.propmanager.feature.auth.AuthViewModel
import com.propmanager.navigation.PropManagerNavHost
import com.propmanager.navigation.Routes
import dagger.hilt.android.AndroidEntryPoint
import javax.inject.Inject

@AndroidEntryPoint
class MainActivity : ComponentActivity() {

    @Inject
    lateinit var networkMonitor: NetworkMonitor

    @Inject
    lateinit var notificacionesRepository: NotificacionesRepository

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            PropManagerTheme {
                PropManagerApp(
                    networkMonitor = networkMonitor,
                    notificacionesRepository = notificacionesRepository,
                )
            }
        }
    }
}

@Composable
private fun PropManagerApp(
    networkMonitor: NetworkMonitor,
    notificacionesRepository: NotificacionesRepository,
    modifier: Modifier = Modifier,
) {
    val navController = rememberNavController()
    val authViewModel: AuthViewModel = hiltViewModel()
    val authState by authViewModel.authState.collectAsStateWithLifecycle()
    val isOnline by networkMonitor.isOnline.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }
    var badgeCount by remember { mutableIntStateOf(0) }

    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentRoute = navBackStackEntry?.destination?.route

    val startDestination = when (authState) {
        is AuthState.Authenticated -> Routes.MAIN_GRAPH
        is AuthState.Unauthenticated -> Routes.AUTH_GRAPH
        is AuthState.Loading -> Routes.AUTH_GRAPH
    }

    val showBottomBar = currentRoute in listOf(
        Routes.DASHBOARD, Routes.PROPIEDADES, Routes.INQUILINOS,
        Routes.CONTRATOS, Routes.MAS,
    )

    LaunchedEffect(authState) {
        if (authState is AuthState.Unauthenticated && currentRoute != Routes.LOGIN) {
            navController.navigate(Routes.AUTH_GRAPH) {
                popUpTo(0) { inclusive = true }
            }
        }
    }

    LaunchedEffect(authState, isOnline) {
        if (authState is AuthState.Authenticated && isOnline) {
            notificacionesRepository.fetchPagosVencidos()
                .onSuccess { badgeCount = it.size }
        }
    }

    Scaffold(
        snackbarHost = { SnackbarHost(snackbarHostState) },
        bottomBar = {
            if (showBottomBar) {
                PropManagerBottomNavBar(
                    currentRoute = currentRoute,
                    onNavigate = { item ->
                        val route = item.route
                        navController.navigate(route) {
                            popUpTo(Routes.DASHBOARD) { saveState = true }
                            launchSingleTop = true
                            restoreState = true
                        }
                    },
                    notificationBadgeCount = badgeCount,
                )
            }
        },
        modifier = modifier,
    ) { paddingValues ->
        PropManagerNavHost(
            navController = navController,
            startDestination = startDestination,
            authViewModel = authViewModel,
            modifier = Modifier.padding(paddingValues),
        )
    }
}
