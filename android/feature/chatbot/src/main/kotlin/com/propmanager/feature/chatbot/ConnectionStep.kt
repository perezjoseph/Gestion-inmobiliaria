package com.propmanager.feature.chatbot

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.propmanager.core.model.dto.ConnectionStatusResponse
import com.propmanager.core.ui.R

@Composable
internal fun ConnectionStep(
    connectionStatus: ConnectionStatusResponse,
    isActionLoading: Boolean,
    onConnect: () -> Unit,
    onDisconnect: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text(
            text = stringResource(R.string.chatbot_step_conexion),
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
        )

        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    StatusIndicator(status = connectionStatus.status)
                    Column {
                        Text(
                            text = stringResource(R.string.chatbot_connection_status),
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Text(
                            text = statusDisplayText(connectionStatus.status),
                            style = MaterialTheme.typography.bodyLarge,
                            fontWeight = FontWeight.Medium,
                        )
                    }
                }

                connectionStatus.connectedPhone?.let { phone ->
                    Spacer(Modifier.height(8.dp))
                    Text(
                        text = stringResource(R.string.chatbot_connected_phone, phone),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            if (connectionStatus.status == "connected") {
                OutlinedButton(
                    onClick = onDisconnect,
                    enabled = !isActionLoading,
                    modifier = Modifier.weight(1f),
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = MaterialTheme.colorScheme.error,
                    ),
                ) {
                    Text(text = stringResource(R.string.chatbot_disconnect))
                }
            } else {
                Button(
                    onClick = onConnect,
                    enabled = !isActionLoading,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(text = stringResource(R.string.chatbot_connect))
                }
            }
        }
    }
}

@Composable
private fun StatusIndicator(
    status: String,
    modifier: Modifier = Modifier,
) {
    val color = when (status) {
        "connected" -> Color(0xFF4CAF50)
        "connecting" -> Color(0xFFFFC107)
        else -> Color(0xFFF44336)
    }
    Box(
        modifier = modifier
            .size(12.dp)
            .clip(CircleShape)
            .background(color),
    )
}

@Composable
private fun statusDisplayText(status: String): String {
    return when (status) {
        "connected" -> stringResource(R.string.chatbot_connected)
        "connecting" -> stringResource(R.string.chatbot_connecting)
        else -> stringResource(R.string.chatbot_disconnected)
    }
}
