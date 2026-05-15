package com.propmanager.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
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
import com.propmanager.feature.inquilinos.InquilinoFormScreen
import com.propmanager.feature.inquilinos.InquilinosListScreen
import com.propmanager.feature.inquilinos.InquilinosViewModel
import com.propmanager.feature.notificaciones.NotificacionesScreen
import com.propmanager.feature.notificaciones.NotificacionesViewModel
import com.propmanager.feature.perfil.PerfilScreen
import com.propmanager.feature.perfil.PerfilViewModel
import com.propmanager.feature.reportes.ReportesScreen
import com.propmanager.feature.reportes.ReportesViewModel
import com.propmanager.feature.propiedades.PropiedadDetailScreen
import com.propmanager.feature.propiedades.PropiedadFormScreen
import com.propmanager.feature.propiedades.PropiedadesListScreen
import com.propmanager.feature.propiedades.PropiedadesViewModel
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
            composable(Routes.LOGIN) { LoginScreen(viewModel = authViewModel) }
        }

        // Main graph
        navigation(startDestination = Routes.DASHBOARD, route = Routes.MAIN_GRAPH) {
            composable(Routes.DASHBOARD) {
                val vm: DashboardViewModel = hiltViewModel()
                DashboardScreen(viewModel = vm)
            }

            // CRUD feature screens
            composable(Routes.PROPIEDADES) {
                val vm: PropiedadesViewModel = hiltViewModel()
                PropiedadesListScreen(
                    viewModel = vm,
                    onNavigateToCreate = { navController.navigate(Routes.propiedadForm()) },
                    onNavigateToDetail = { id -> navController.navigate(Routes.propiedadDetail(id)) },
                )
            }
            composable(Routes.PROPIEDAD_DETAIL) { backStackEntry ->
                val id = backStackEntry.arguments?.getString("id") ?: return@composable
                val vm: PropiedadesViewModel = hiltViewModel()
                PropiedadDetailScreen(
                    viewModel = vm,
                    propiedadId = id,
                    onNavigateBack = { navController.popBackStack() },
                    onNavigateToEdit = { navController.navigate(Routes.propiedadForm(id)) },
                )
            }
            composable(Routes.PROPIEDAD_FORM) { backStackEntry ->
                val id = backStackEntry.arguments?.getString("id")?.takeIf { it.isNotEmpty() }
                val vm: PropiedadesViewModel = hiltViewModel()
                LaunchedEffect(id) { id?.let { vm.loadDetail(it) } }
                PropiedadFormScreen(
                    viewModel = vm,
                    isEditing = id != null,
                    onNavigateBack = { navController.popBackStack() },
                )
            }
            composable(Routes.INQUILINOS) {
                val vm: InquilinosViewModel = hiltViewModel()
                InquilinosListScreen(
                    viewModel = vm,
                    onNavigateToCreate = {
                        vm.initCreateForm()
                        navController.navigate(Routes.inquilinoForm())
                    },
                    onNavigateToEdit = { id -> navController.navigate(Routes.inquilinoForm(id)) },
                )
            }
            composable(Routes.INQUILINO_FORM) { backStackEntry ->
                val id = backStackEntry.arguments?.getString("id")?.takeIf { it.isNotEmpty() }
                val vm: InquilinosViewModel = hiltViewModel()
                val savedStateHandle = backStackEntry.savedStateHandle

                LaunchedEffect(id) { id?.let { vm.loadEdit(it) } }

                // Receive OCR results from scanner
                LaunchedEffect(Unit) {
                    savedStateHandle.getStateFlow("cedula_nombre", "").collect { nombre ->
                        if (nombre.isNotEmpty()) {
                            val apellido = savedStateHandle.get<String>("cedula_apellido") ?: ""
                            val numero = savedStateHandle.get<String>("cedula_numero") ?: ""
                            vm.prefillFromOcr(nombre, apellido, numero)
                            savedStateHandle["cedula_nombre"] = ""
                            savedStateHandle["cedula_apellido"] = ""
                            savedStateHandle["cedula_numero"] = ""
                        }
                    }
                }

                InquilinoFormScreen(
                    viewModel = vm,
                    isEditing = id != null,
                    onNavigateBack = { navController.popBackStack() },
                    onScanCedula = { navController.navigate(Routes.SCANNER_CEDULA) },
                )
            }
            composable(Routes.CONTRATOS) { /* ContratosListScreen */ }
            composable(Routes.PAGOS) { /* PagosListScreen */ }
            composable(Routes.GASTOS) { /* GastosListScreen */ }
            composable(Routes.MANTENIMIENTO) { /* MantenimientoListScreen */ }

            // Online-only features
            composable(Routes.REPORTES) {
                val vm: ReportesViewModel = hiltViewModel()
                ReportesScreen(viewModel = vm, onNavigateBack = { navController.popBackStack() })
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
                AuditoriaScreen(viewModel = vm, onNavigateBack = { navController.popBackStack() })
            }

            composable(Routes.PERFIL) {
                val vm: PerfilViewModel = hiltViewModel()
                PerfilScreen(viewModel = vm, onNavigateBack = { navController.popBackStack() })
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
                val entityType =
                    backStackEntry.arguments?.getString("entityType") ?: return@composable
                val entityId = backStackEntry.arguments?.getString("entityId") ?: return@composable
                val vm: DocumentosViewModel = hiltViewModel()
                LaunchedEffect(entityType, entityId) { vm.loadDocuments(entityType, entityId) }
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
