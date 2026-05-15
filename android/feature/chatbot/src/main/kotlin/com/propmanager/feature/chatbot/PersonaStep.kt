package com.propmanager.feature.chatbot

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.propmanager.core.model.dto.ChatbotConfigResponse
import com.propmanager.core.model.dto.ChatbotConfigUpdateRequest
import com.propmanager.core.ui.R

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun PersonaStep(
    config: ChatbotConfigResponse,
    isActionLoading: Boolean,
    onSave: (ChatbotConfigUpdateRequest) -> Unit,
    modifier: Modifier = Modifier,
) {
    var displayName by remember(config.id) { mutableStateOf(config.displayName.orEmpty()) }
    var language by remember(config.id) { mutableStateOf(config.language) }
    var tone by remember(config.id) { mutableStateOf(config.tone.orEmpty()) }
    var greeting by remember(config.id) { mutableStateOf(config.greeting.orEmpty()) }
    var systemPrompt by remember(config.id) { mutableStateOf(config.systemPrompt.orEmpty()) }

    val languages = remember { listOf("es", "en") }
    val tones = remember { listOf("formal", "amigable", "profesional") }

    Column(
        modifier = modifier
            .padding(16.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            text = stringResource(R.string.chatbot_step_personalidad),
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
        )

        OutlinedTextField(
            value = displayName,
            onValueChange = { displayName = it },
            label = { Text(stringResource(R.string.chatbot_display_name)) },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )

        DropdownSelector(
            label = stringResource(R.string.chatbot_language),
            selectedValue = language,
            options = languages,
            onOptionSelected = { language = it },
        )

        DropdownSelector(
            label = stringResource(R.string.chatbot_tone),
            selectedValue = tone,
            options = tones,
            onOptionSelected = { tone = it },
        )

        OutlinedTextField(
            value = greeting,
            onValueChange = { greeting = it },
            label = { Text(stringResource(R.string.chatbot_greeting)) },
            minLines = 3,
            maxLines = 5,
            modifier = Modifier.fillMaxWidth(),
        )

        OutlinedTextField(
            value = systemPrompt,
            onValueChange = { systemPrompt = it },
            label = { Text(stringResource(R.string.chatbot_system_prompt)) },
            minLines = 4,
            maxLines = 8,
            modifier = Modifier.fillMaxWidth(),
        )

        Button(
            onClick = {
                onSave(
                    ChatbotConfigUpdateRequest(
                        displayName = displayName,
                        language = language,
                        tone = tone.ifBlank { null },
                        greeting = greeting.ifBlank { null },
                        systemPrompt = systemPrompt.ifBlank { null },
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DropdownSelector(
    label: String,
    selectedValue: String,
    options: List<String>,
    onOptionSelected: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }

    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it },
        modifier = modifier,
    ) {
        OutlinedTextField(
            value = selectedValue,
            onValueChange = {},
            readOnly = true,
            label = { Text(label) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier = Modifier
                .fillMaxWidth()
                .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable),
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
        ) {
            options.forEach { option ->
                DropdownMenuItem(
                    text = { Text(option) },
                    onClick = {
                        expanded = false
                        onOptionSelected(option)
                    },
                )
            }
        }
    }
}
