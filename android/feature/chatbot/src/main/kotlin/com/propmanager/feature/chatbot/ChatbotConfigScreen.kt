package com.propmanager.feature.chatbot

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.ErrorScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.PropManagerTopAppBar

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatbotConfigScreen(
    viewModel: ChatbotConfigViewModel,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val chatMessages by viewModel.chatMessages.collectAsStateWithLifecycle()
    val actionError by viewModel.actionError.collectAsStateWithLifecycle()
    val isActionLoading by viewModel.isActionLoading.collectAsStateWithLifecycle()
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
                title = stringResource(R.string.nav_chatbot),
                scrollBehavior = scrollBehavior,
                onNavigateBack = onNavigateBack,
            )
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues),
        ) {
            when (val state = uiState) {
                is ChatbotConfigUiState.Loading -> LoadingScreen()
                is ChatbotConfigUiState.Error -> {
                    ErrorScreen(
                        message = state.message,
                        onRetry = remember { { viewModel.loadConfig() } },
                    )
                }
                is ChatbotConfigUiState.Success -> {
                    ChatbotWizardContent(
                        state = state,
                        chatMessages = chatMessages,
                        isActionLoading = isActionLoading,
                        onNextStep = remember { { viewModel.nextStep() } },
                        onPreviousStep = remember { { viewModel.previousStep() } },
                        onUpdateConfig = viewModel::updateConfig,
                        onConnect = remember { { viewModel.connect() } },
                        onDisconnect = remember { { viewModel.disconnect() } },
                        onSendMessage = viewModel::testChat,
                        onClearChat = remember { { viewModel.clearChatHistory() } },
                    )
                }
            }
        }
    }
}

@Composable
private fun ChatbotWizardContent(
    state: ChatbotConfigUiState.Success,
    chatMessages: kotlinx.collections.immutable.ImmutableList<ChatMessage>,
    isActionLoading: Boolean,
    onNextStep: () -> Unit,
    onPreviousStep: () -> Unit,
    onUpdateConfig: (com.propmanager.core.model.dto.ChatbotConfigUpdateRequest) -> Unit,
    onConnect: () -> Unit,
    onDisconnect: () -> Unit,
    onSendMessage: (String) -> Unit,
    onClearChat: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val stepLabels = remember {
        listOf(
            R.string.chatbot_step_conexion,
            R.string.chatbot_step_personalidad,
            R.string.chatbot_step_capacidades,
            R.string.chatbot_step_conocimiento,
            R.string.chatbot_step_remitentes,
            R.string.chatbot_step_prueba,
            R.string.chatbot_step_activar,
        )
    }

    Column(modifier = modifier.fillMaxSize()) {
        // Step indicator (scrollable tab row)
        PrimaryScrollableTabRow(
            selectedTabIndex = state.currentStep,
            edgePadding = 8.dp,
        ) {
            stepLabels.forEachIndexed { index, labelRes ->
                Tab(
                    selected = state.currentStep == index,
                    onClick = {},
                    text = {
                        Text(
                            text = stringResource(labelRes),
                            style = MaterialTheme.typography.labelMedium,
                        )
                    },
                )
            }
        }

        // Content area
        Column(modifier = Modifier.weight(1f)) {
            when (state.currentStep) {
                0 -> ConnectionStep(
                    connectionStatus = state.connectionStatus,
                    isActionLoading = isActionLoading,
                    onConnect = onConnect,
                    onDisconnect = onDisconnect,
                )
                1 -> PersonaStep(
                    config = state.config,
                    isActionLoading = isActionLoading,
                    onSave = onUpdateConfig,
                )
                2 -> CapabilitiesStep(
                    capabilities = state.config.capabilities,
                    isActionLoading = isActionLoading,
                    onSave = onUpdateConfig,
                )
                3 -> KnowledgeStep(
                    config = state.config,
                    isActionLoading = isActionLoading,
                    onSave = onUpdateConfig,
                )
                4 -> SenderPolicyStep(
                    config = state.config,
                    isActionLoading = isActionLoading,
                    onSave = onUpdateConfig,
                )
                5 -> TestChatStep(
                    messages = chatMessages,
                    isActionLoading = isActionLoading,
                    onSendMessage = onSendMessage,
                    onClearChat = onClearChat,
                )
                6 -> ActivationStep(
                    config = state.config,
                    connectionStatus = state.connectionStatus,
                    isActionLoading = isActionLoading,
                    onSave = onUpdateConfig,
                )
            }
        }

        // Navigation buttons
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
        ) {
            if (state.currentStep > 0) {
                OutlinedButton(
                    onClick = onPreviousStep,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(text = stringResource(R.string.chatbot_previous))
                }
                Spacer(Modifier.width(12.dp))
            }
            if (state.currentStep < ChatbotConfigViewModel.TOTAL_STEPS - 1) {
                Button(
                    onClick = onNextStep,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(text = stringResource(R.string.chatbot_next))
                }
            }
        }
    }
}
