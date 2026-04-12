package com.propmanager.navigation

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.navigation
import com.propmanager.feature.auditoria.AuditoriaScreen
import com.propmanager.feature.auditoria.AuditoriaViewModel
import com.propmanager.feature.auth.AuthViewModel
import com.propmanager.feature.auth.LoginScreen
import com.propmanager.feature.configuracion.ConfiguracionScreen
import com.propmanager.feature.configuracion.ConfiguracionViewModel
import com.propmanager.feature.dashboard.DashboardScreen
import com.propmanager.feature.dashboard.DashboardViewModel
import com.propmanager.feature.documentos.DocumentosScreen
import com.propmanager.feature.documentos.DocumentosViewModel
import com.propmanager.feature.importacion.ImportacionScreen
import com.propmanager.feature.importacion.ImportacionViewModel
import com.propmanager.feature.notificaciones.NotificacionesScreen
import com.propmanager.feature.notificaciones.NotificacionesViewModel
import com.propmanager.feature.perfil.PerfilScreen
import com.propmanager.feature.perfil.PerfilViewModel
import com.propmanager.feature.reportes.ReportesScreen
import com.propmanager.feature.reportes.ReportesViewModel
import com.propmanager.feature.scanner.ScannerMode
import com.propmanager.feature.scanner.ScannerScreen
import com.propmanager.feature.scanner.ScannerViewModel

@Composable
fun PropManagerNavHost(
    navController: NavHostController,
    startDestination: String,
    authViewModel: AuthViewModel,
    modifier: Modifier = Modifier,
) {
    NavHost(
        navController = navController,
        startDestination = startDestination,
        modifier = modifier,
    ) {
        // Auth graph
        navigation(startDestination = Routes.LOGIN, route = Routes.AUTH_GRAPH) {
            composable(Routes.LOGIN) {
                LoginScreen(viewModel = authViewModel)
            }
        }

        // Main graph
        navigation(startDestination = Routes.DASHBOARD, route = Routes.MAIN_GRAPH) {
            composable(Routes.DASHBOARD) {
                val vm: DashboardViewModel = hiltViewModel()
                DashboardScreen(viewModel = vm)
            }

            // Placeholder routes for offline-first CRUD features
            // (Propiedades, Inquilinos, Contratos, Pagos, Gastos, Mantenimiento)
            // These will be wired to their respective screens with full navigation
            composable(Routes.PROPIEDADES) { /* PropiedadesListScreen */ }
            composable(Routes.INQUILINOS) { /* InquilinosListScreen */ }
            composable(Routes.CONTRATOS) { /* ContratosListScreen */ }
            composable(Routes.PAGOS) { /* PagosListScreen */ }
            composable(Routes.GASTOS) { /* GastosListScreen */ }
            composable(Routes.MANTENIMIENTO) { /* MantenimientoListScreen */ }

            // Online-only features
            composable(Routes.REPORTES) {
                val vm: ReportesViewModel = hiltViewModel()
                ReportesScreen(
                    viewModel = vm,
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            composable(Routes.NOTIFICACIONES) {
                val vm: NotificacionesViewModel = hiltViewModel()
                NotificacionesScreen(
                    viewModel = vm,
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            composable(Routes.AUDITORIA) {
                val vm: AuditoriaViewModel = hiltViewModel()
                AuditoriaScreen(
                    viewModel = vm,
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            composable(Routes.PERFIL) {
                val vm: PerfilViewModel = hiltViewModel()
                PerfilScreen(
                    viewModel = vm,
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            composable(Routes.CONFIGURACION) {
                val vm: ConfiguracionViewModel = hiltViewModel()
                ConfiguracionScreen(
                    viewModel = vm,
                    onNavigateBack = { navController.popBackStack() },
                )
            }

            composable(Routes.IMPORTACION) {
                val vm: ImportacionViewModel = hiltViewModel()
                ImportacionScreen(
                    viewModel = vm,
                    onNavigateBack = { navController.popBackStack() },
                    onPickFile = { /* File picker intent will be handled by Activity result */ },
                )
            }

            composable("documentos/{entityType}/{entityId}") { backStackEntry ->
                val entityType = backStackEntry.arguments?.getString("entityType") ?: return@composable
                val entityId = backStackEntry.arguments?.getString("entityId") ?: return@composable
                val vm: DocumentosViewModel = hiltViewModel()
                vm.loadDocuments(entityType, entityId)
                DocumentosScreen(
                    viewModel = vm,
                    onNavigateBack = { navController.popBackStack() },
                    onPickFile = { /* File picker intent */ },
                )
            }

            // Scanner routes
            composable(Routes.SCANNER_CEDULA) {
                val vm: ScannerViewModel = hiltViewModel()
                ScannerScreen(
                    viewModel = vm,
                    mode = ScannerMode.CEDULA,
                    onNavigateBack = { navController.popBackStack() },
                    onConfirmCedula = { result ->
                        navController.previousBackStackEntry?.savedStateHandle?.apply {
                            set("cedula_nombre", result.nombre ?: "")
                            set("cedula_apellido", result.apellido ?: "")
                            set("cedula_numero", result.cedula ?: "")
                        }
                        navController.popBackStack()
                    },
                    onConfirmReceipt = {},
                )
            }

            composable(Routes.SCANNER_RECEIPT) {
                val vm: ScannerViewModel = hiltViewModel()
                ScannerScreen(
                    viewModel = vm,
                    mode = ScannerMode.RECEIPT,
                    onNavigateBack = { navController.popBackStack() },
                    onConfirmCedula = {},
                    onConfirmReceipt = { result ->
                        navController.previousBackStackEntry?.savedStateHandle?.apply {
                            set("receipt_monto", result.monto?.toPlainString() ?: "")
                            set("receipt_fecha", result.fecha?.toString() ?: "")
                            set("receipt_proveedor", result.proveedor ?: "")
                            set("receipt_factura", result.numeroFactura ?: "")
                        }
                        navController.popBackStack()
                    },
                )
            }

            // "Más" overflow menu screen
            composable(Routes.MAS) {
                MasMenuScreen(
                    onNavigate = { route -> navController.navigate(route) },
                    onNavigateBack = { navController.popBackStack() },
                )
            }
        }
    }
}
