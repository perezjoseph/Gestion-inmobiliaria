package com.propmanager.feature.chatbot

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.propmanager.core.model.dto.ChatbotConfigResponse
import com.propmanager.core.model.dto.ChatbotConfigUpdateRequest
import com.propmanager.core.ui.R

@Composable
internal fun SenderPolicyStep(
    config: ChatbotConfigResponse,
    isActionLoading: Boolean,
    onSave: (ChatbotConfigUpdateRequest) -> Unit,
    modifier: Modifier = Modifier,
) {
    var senderPolicy by remember(config.id) { mutableStateOf(config.senderPolicy) }
    var allowlist by remember(config.id) { mutableStateOf(config.allowlist.orEmpty()) }
    var newPhone by remember { mutableStateOf("") }

    val policyOptions = remember {
        listOf("all", "allowlist", "inquilinos_only")
    }

    Column(
        modifier = modifier
            .padding(16.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text(
            text = stringResource(R.string.chatbot_step_remitentes),
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
        )

        Text(
            text = stringResource(R.string.chatbot_sender_policy),
            style = MaterialTheme.typography.titleMedium,
        )

        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(16.dp)) {
                policyOptions.forEach { option ->
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(
                            selected = senderPolicy == option,
                            onClick = { senderPolicy = option },
                        )
                        Text(
                            text = policyLabel(option),
                            style = MaterialTheme.typography.bodyLarge,
                            modifier = Modifier.padding(start = 8.dp),
                        )
                    }
                }
            }
        }

        if (senderPolicy == "allowlist") {
            Text(
                text = stringResource(R.string.chatbot_allowlist_phones),
                style = MaterialTheme.typography.titleMedium,
            )

            allowlist.forEachIndexed { index, phone ->
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = phone,
                        style = MaterialTheme.typography.bodyLarge,
                        modifier = Modifier.weight(1f),
                    )
                    IconButton(
                        onClick = {
                            allowlist = allowlist.toMutableList().also { it.removeAt(index) }
                        },
                    ) {
                        Icon(
                            Icons.Default.Delete,
                            contentDescription = stringResource(R.string.delete),
                            tint = MaterialTheme.colorScheme.error,
                        )
                    }
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                OutlinedTextField(
                    value = newPhone,
                    onValueChange = { newPhone = it },
                    label = { Text(stringResource(R.string.chatbot_phone_number)) },
                    singleLine = true,
                    modifier = Modifier.weight(1f),
                )
                TextButton(
                    onClick = {
                        if (newPhone.isNotBlank()) {
                            allowlist = allowlist + newPhone.trim()
                            newPhone = ""
                        }
                    },
                ) {
                    Icon(Icons.Default.Add, contentDescription = null)
                    Text(text = stringResource(R.string.chatbot_add_phone))
                }
            }
        }

        Spacer(Modifier.height(8.dp))

        Button(
            onClick = {
                onSave(
                    ChatbotConfigUpdateRequest(
                        senderPolicy = senderPolicy,
                        allowlist = if (senderPolicy == "allowlist") allowlist else null,
                    ),
                )
            },
            enabled = !isActionLoading,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(text = stringResource(R.string.save))
        }
    }
}

@Composable
private fun policyLabel(policy: String): String {
    return when (policy) {
        "all" -> stringResource(R.string.chatbot_sender_all)
        "allowlist" -> stringResource(R.string.chatbot_sender_allowlist)
        "inquilinos_only" -> stringResource(R.string.chatbot_sender_inquilinos_only)
        else -> policy
    }
}
