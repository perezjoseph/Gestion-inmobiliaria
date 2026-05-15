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
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
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
import com.propmanager.core.model.dto.FaqEntry
import com.propmanager.core.ui.R

@Composable
internal fun KnowledgeStep(
    config: ChatbotConfigResponse,
    isActionLoading: Boolean,
    onSave: (ChatbotConfigUpdateRequest) -> Unit,
    modifier: Modifier = Modifier,
) {
    var faqs by remember(config.id) { mutableStateOf(config.faqs.orEmpty()) }
    var policies by remember(config.id) { mutableStateOf(config.policies.orEmpty()) }
    var deleteConfirmIndex by remember { mutableIntStateOf(-1) }

    if (deleteConfirmIndex >= 0) {
        val faqToDelete = faqs.getOrNull(deleteConfirmIndex)
        if (faqToDelete != null) {
            AlertDialog(
                onDismissRequest = { deleteConfirmIndex = -1 },
                title = { Text(stringResource(R.string.confirm_delete_title)) },
                text = {
                    Text(
                        stringResource(
                            R.string.confirm_delete_message,
                            faqToDelete.question.ifBlank { "FAQ" },
                        ),
                    )
                },
                confirmButton = {
                    TextButton(
                        onClick = {
                            faqs = faqs.toMutableList().also { it.removeAt(deleteConfirmIndex) }
                            deleteConfirmIndex = -1
                        },
                    ) {
                        Text(stringResource(R.string.delete))
                    }
                },
                dismissButton = {
                    TextButton(onClick = { deleteConfirmIndex = -1 }) {
                        Text(stringResource(R.string.cancel))
                    }
                },
            )
        }
    }

    Column(
        modifier = modifier
            .padding(16.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text(
            text = stringResource(R.string.chatbot_step_conocimiento),
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
        )

        // FAQ Section
        Text(
            text = stringResource(R.string.chatbot_faqs),
            style = MaterialTheme.typography.titleMedium,
        )

        faqs.forEachIndexed { index, faq ->
            FaqEntryCard(
                faq = faq,
                onQuestionChange = { newQuestion ->
                    faqs = faqs.toMutableList().also {
                        it[index] = it[index].copy(question = newQuestion)
                    }
                },
                onAnswerChange = { newAnswer ->
                    faqs = faqs.toMutableList().also {
                        it[index] = it[index].copy(answer = newAnswer)
                    }
                },
                onDelete = { deleteConfirmIndex = index },
            )
        }

        TextButton(
            onClick = {
                faqs = faqs + FaqEntry(question = "", answer = "")
            },
        ) {
            Icon(Icons.Default.Add, contentDescription = null)
            Text(text = stringResource(R.string.chatbot_faq_add))
        }

        Spacer(Modifier.height(8.dp))

        // Policies Section
        Text(
            text = stringResource(R.string.chatbot_policies),
            style = MaterialTheme.typography.titleMedium,
        )

        OutlinedTextField(
            value = policies,
            onValueChange = { policies = it },
            label = { Text(stringResource(R.string.chatbot_policies)) },
            minLines = 5,
            maxLines = 10,
            modifier = Modifier.fillMaxWidth(),
        )

        Button(
            onClick = {
                val validFaqs = faqs.filter { it.question.isNotBlank() && it.answer.isNotBlank() }
                onSave(
                    ChatbotConfigUpdateRequest(
                        faqs = validFaqs,
                        policies = policies.ifBlank { null },
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
private fun FaqEntryCard(
    faq: FaqEntry,
    onQuestionChange: (String) -> Unit,
    onAnswerChange: (String) -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Card(modifier = modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(12.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedTextField(
                    value = faq.question,
                    onValueChange = onQuestionChange,
                    label = { Text(stringResource(R.string.chatbot_faq_question)) },
                    singleLine = true,
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = onDelete) {
                    Icon(
                        Icons.Default.Delete,
                        contentDescription = stringResource(R.string.delete),
                        tint = MaterialTheme.colorScheme.error,
                    )
                }
            }
            Spacer(Modifier.height(8.dp))
            OutlinedTextField(
                value = faq.answer,
                onValueChange = onAnswerChange,
                label = { Text(stringResource(R.string.chatbot_faq_answer)) },
                minLines = 2,
                maxLines = 4,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}
