package com.propmanager.feature.chatbot

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.propmanager.core.model.dto.Capabilities
import com.propmanager.core.model.dto.ChatbotConfigUpdateRequest
import com.propmanager.core.ui.R

@Composable
internal fun CapabilitiesStep(
    capabilities: Capabilities,
    isActionLoading: Boolean,
    onSave: (ChatbotConfigUpdateRequest) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text(
            text = stringResource(R.string.chatbot_step_capacidades),
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
        )

        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(16.dp)) {
                CapabilityToggle(
                    label = stringResource(R.string.chatbot_capability_receipt_ocr),
                    checked = capabilities.receiptOcr,
                    enabled = !isActionLoading,
                    onCheckedChange = { checked ->
                        onSave(
                            ChatbotConfigUpdateRequest(
                                capabilities = capabilities.copy(receiptOcr = checked),
                            ),
                        )
                    },
                )
                CapabilityToggle(
                    label = stringResource(R.string.chatbot_capability_balance),
                    checked = capabilities.balanceQueries,
                    enabled = !isActionLoading,
                    onCheckedChange = { checked ->
                        onSave(
                            ChatbotConfigUpdateRequest(
                                capabilities = capabilities.copy(balanceQueries = checked),
                            ),
                        )
                    },
                )
                CapabilityToggle(
                    label = stringResource(R.string.chatbot_capability_reminders),
                    checked = capabilities.paymentReminders,
                    enabled = !isActionLoading,
                    onCheckedChange = { checked ->
                        onSave(
                            ChatbotConfigUpdateRequest(
                                capabilities = capabilities.copy(paymentReminders = checked),
                            ),
                        )
                    },
                )
                CapabilityToggle(
                    label = stringResource(R.string.chatbot_capability_maintenance),
                    checked = capabilities.maintenanceRequests,
                    enabled = !isActionLoading,
                    onCheckedChange = { checked ->
                        onSave(
                            ChatbotConfigUpdateRequest(
                                capabilities = capabilities.copy(maintenanceRequests = checked),
                            ),
                        )
                    },
                )
                CapabilityToggle(
                    label = stringResource(R.string.chatbot_capability_handoff),
                    checked = capabilities.humanHandoff,
                    enabled = !isActionLoading,
                    onCheckedChange = { checked ->
                        onSave(
                            ChatbotConfigUpdateRequest(
                                capabilities = capabilities.copy(humanHandoff = checked),
                            ),
                        )
                    },
                )
            }
        }
    }
}

@Composable
private fun CapabilityToggle(
    label: String,
    checked: Boolean,
    enabled: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyLarge,
            modifier = Modifier.weight(1f),
        )
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            enabled = enabled,
        )
    }
}
