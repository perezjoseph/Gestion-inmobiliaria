package com.propmanager.feature.chatbot

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.propmanager.core.model.dto.ChatbotConfigResponse
import com.propmanager.core.model.dto.ChatbotConfigUpdateRequest
import com.propmanager.core.model.dto.ConnectionStatusResponse
import com.propmanager.core.ui.R

@Composable
internal fun ActivationStep(
    config: ChatbotConfigResponse,
    connectionStatus: ConnectionStatusResponse,
    isActionLoading: Boolean,
    onSave: (ChatbotConfigUpdateRequest) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text(
            text = stringResource(R.string.chatbot_step_activar),
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
        )

        // Large toggle
        Card(modifier = Modifier.fillMaxWidth()) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(24.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column {
                    Text(
                        text = stringResource(R.string.chatbot_activation),
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Medium,
                    )
                    Text(
                        text = if (config.activo) {
                            stringResource(R.string.chatbot_active)
                        } else {
                            stringResource(R.string.chatbot_inactive)
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Switch(
                    checked = config.activo,
                    onCheckedChange = { checked ->
                        onSave(ChatbotConfigUpdateRequest(activo = checked))
                    },
                    enabled = !isActionLoading,
                )
            }
        }

        // Warning
        if (!config.activo) {
            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.errorContainer,
                ),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    modifier = Modifier.padding(16.dp),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalAlignment = Alignment.Top,
                ) {
                    Icon(
                        Icons.Default.Warning,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onErrorContainer,
                    )
                    Text(
                        text = stringResource(R.string.chatbot_activation_warning),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onErrorContainer,
                    )
                }
            }
        }

        Spacer(Modifier.height(8.dp))

        // Configuration summary
        Text(
            text = stringResource(R.string.chatbot_config_summary),
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Medium,
        )

        Card(modifier = Modifier.fillMaxWidth()) {
            Column(
                modifier = Modifier.padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                SummaryRow(
                    label = stringResource(R.string.chatbot_connection_status),
                    value = connectionStatus.status,
                )
                SummaryRow(
                    label = stringResource(R.string.chatbot_display_name),
                    value = config.displayName.orEmpty().ifBlank { "—" },
                )
                SummaryRow(
                    label = stringResource(R.string.chatbot_language),
                    value = config.language,
                )
                SummaryRow(
                    label = stringResource(R.string.chatbot_sender_policy),
                    value = config.senderPolicy,
                )
                SummaryRow(
                    label = stringResource(R.string.chatbot_faqs),
                    value = "${config.faqs?.size ?: 0}",
                )
            }
        }
    }
}

@Composable
private fun SummaryRow(
    label: String,
    value: String,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.Medium,
        )
    }
}
