package com.propmanager.feature.chatbot

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
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.propmanager.core.model.dto.ReceiptExtractionResponse
import com.propmanager.core.ui.R
import com.propmanager.core.ui.components.EmptyStateScreen
import com.propmanager.core.ui.components.LoadingScreen
import com.propmanager.core.ui.components.PropManagerTopAppBar
import kotlinx.collections.immutable.ImmutableList

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PendingReceiptsScreen(
    viewModel: ChatbotConfigViewModel,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val pendingReceipts by viewModel.pendingReceipts.collectAsStateWithLifecycle()
    val actionError by viewModel.actionError.collectAsStateWithLifecycle()
    val isActionLoading by viewModel.isActionLoading.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }
    val scrollBehavior = TopAppBarDefaults.pinnedScrollBehavior()

    LaunchedEffect(Unit) { viewModel.loadPendingReceipts() }

    LaunchedEffect(actionError) {
        actionError?.let {
            snackbarHostState.showSnackbar(it)
            viewModel.clearActionError()
        }
    }

    Scaffold(
        topBar = {
            PropManagerTopAppBar(
                title = stringResource(R.string.chatbot_pending_receipts_title),
                scrollBehavior = scrollBehavior,
                onNavigateBack = onNavigateBack,
            )
        },
        snackbarHost = { SnackbarHost(snackbarHostState) },
        modifier = modifier.nestedScroll(scrollBehavior.nestedScrollConnection),
    ) { paddingValues ->
        Column(modifier = Modifier.fillMaxSize().padding(paddingValues)) {
            PendingReceiptsContent(
                receipts = pendingReceipts,
                isActionLoading = isActionLoading,
                onConfirm = viewModel::confirmReceipt,
                onReject = viewModel::rejectReceipt,
            )
        }
    }
}

@Composable
private fun PendingReceiptsContent(
    receipts: ImmutableList<ReceiptExtractionResponse>,
    isActionLoading: Boolean,
    onConfirm: (String, String?) -> Unit,
    onReject: (String, String?) -> Unit,
    modifier: Modifier = Modifier,
) {
    if (receipts.isEmpty()) {
        EmptyStateScreen(message = stringResource(R.string.chatbot_pending_receipts_empty))
    } else {
        LazyColumn(
            modifier = modifier.fillMaxSize().padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item { Spacer(Modifier.height(4.dp)) }
            items(receipts, key = { it.id }) { receipt ->
                ReceiptCard(
                    receipt = receipt,
                    isActionLoading = isActionLoading,
                    onConfirm = onConfirm,
                    onReject = onReject,
                )
            }
            item { Spacer(Modifier.height(16.dp)) }
        }
    }
}

@Composable
private fun ReceiptCard(
    receipt: ReceiptExtractionResponse,
    isActionLoading: Boolean,
    onConfirm: (String, String?) -> Unit,
    onReject: (String, String?) -> Unit,
    modifier: Modifier = Modifier,
) {
    var showConfirmDialog by remember { mutableStateOf(false) }
    var showRejectDialog by remember { mutableStateOf(false) }

    if (showConfirmDialog) {
        ConfirmReceiptDialog(
            receipt = receipt,
            onConfirm = { contratoId ->
                showConfirmDialog = false
                onConfirm(receipt.id, contratoId)
            },
            onDismiss = { showConfirmDialog = false },
        )
    }

    if (showRejectDialog) {
        RejectReceiptDialog(
            onReject = { reason ->
                showRejectDialog = false
                onReject(receipt.id, reason)
            },
            onDismiss = { showRejectDialog = false },
        )
    }

    Card(
        modifier = modifier.fillMaxWidth(),
        elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            // Header: sender + confidence
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = receipt.senderName ?: stringResource(R.string.chatbot_receipt_unknown_sender),
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Medium,
                )
                ConfidenceBadge(confidence = receipt.confidence)
            }

            Spacer(Modifier.height(8.dp))

            // Amount + currency
            Text(
                text = "${receipt.currency} ${receipt.amount}",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.primary,
            )

            Spacer(Modifier.height(8.dp))

            // Details
            receipt.bank?.let { bank ->
                DetailRow(
                    label = stringResource(R.string.chatbot_receipt_bank),
                    value = bank,
                )
            }
            receipt.date?.let { date ->
                DetailRow(
                    label = stringResource(R.string.chatbot_receipt_date),
                    value = date,
                )
            }
            receipt.reference?.let { reference ->
                DetailRow(
                    label = stringResource(R.string.chatbot_receipt_reference),
                    value = reference,
                )
            }

            Spacer(Modifier.height(12.dp))

            // Action buttons
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
            ) {
                OutlinedButton(
                    onClick = { showRejectDialog = true },
                    enabled = !isActionLoading,
                ) {
                    Text(stringResource(R.string.chatbot_receipt_reject))
                }
                Spacer(Modifier.width(8.dp))
                Button(
                    onClick = { showConfirmDialog = true },
                    enabled = !isActionLoading,
                ) {
                    Text(stringResource(R.string.chatbot_receipt_confirm))
                }
            }
        }
    }
}

@Composable
private fun DetailRow(
    label: String,
    value: String,
    modifier: Modifier = Modifier,
) {
    Row(modifier = modifier.padding(vertical = 2.dp)) {
        Text(
            text = "$label: ",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodySmall,
        )
    }
}

@Composable
private fun ConfidenceBadge(
    confidence: String,
    modifier: Modifier = Modifier,
) {
    val color = when {
        confidence.startsWith("high", ignoreCase = true) ||
            confidence.toDoubleOrNull()?.let { it >= 0.8 } == true ->
            MaterialTheme.colorScheme.primary

        confidence.startsWith("medium", ignoreCase = true) ||
            confidence.toDoubleOrNull()?.let { it >= 0.5 } == true ->
            MaterialTheme.colorScheme.tertiary

        else -> MaterialTheme.colorScheme.error
    }

    Text(
        text = confidence,
        style = MaterialTheme.typography.labelSmall,
        color = color,
        fontWeight = FontWeight.Medium,
        modifier = modifier,
    )
}

@Composable
private fun ConfirmReceiptDialog(
    receipt: ReceiptExtractionResponse,
    onConfirm: (String?) -> Unit,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var contratoId by remember { mutableStateOf(receipt.contratoId ?: "") }

    AlertDialog(
        onDismissRequest = onDismiss,
        modifier = modifier,
        title = { Text(stringResource(R.string.chatbot_receipt_confirm_title)) },
        text = {
            Column {
                Text(
                    text = stringResource(
                        R.string.chatbot_receipt_confirm_message,
                        receipt.currency,
                        receipt.amount,
                    ),
                    style = MaterialTheme.typography.bodyMedium,
                )
                Spacer(Modifier.height(12.dp))
                OutlinedTextField(
                    value = contratoId,
                    onValueChange = { contratoId = it },
                    label = { Text(stringResource(R.string.chatbot_receipt_contrato_id)) },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                )
                Text(
                    text = stringResource(R.string.chatbot_receipt_contrato_optional),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        confirmButton = {
            TextButton(onClick = { onConfirm(contratoId.takeIf { it.isNotBlank() }) }) {
                Text(stringResource(R.string.confirm))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.cancel))
            }
        },
    )
}

@Composable
private fun RejectReceiptDialog(
    onReject: (String?) -> Unit,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var reason by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = onDismiss,
        modifier = modifier,
        title = { Text(stringResource(R.string.chatbot_receipt_reject_title)) },
        text = {
            Column {
                Text(
                    text = stringResource(R.string.chatbot_receipt_reject_message),
                    style = MaterialTheme.typography.bodyMedium,
                )
                Spacer(Modifier.height(12.dp))
                OutlinedTextField(
                    value = reason,
                    onValueChange = { reason = it },
                    label = { Text(stringResource(R.string.chatbot_receipt_reject_reason)) },
                    modifier = Modifier.fillMaxWidth(),
                    minLines = 2,
                    maxLines = 4,
                )
            }
        },
        confirmButton = {
            TextButton(onClick = { onReject(reason.takeIf { it.isNotBlank() }) }) {
                Text(stringResource(R.string.chatbot_receipt_reject))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.cancel))
            }
        },
    )
}
