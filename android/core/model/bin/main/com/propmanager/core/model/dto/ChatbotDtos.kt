package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class FaqEntry(
    val question: String,
    val answer: String,
)

@Serializable
data class Capabilities(
    @SerialName("receiptOcr") val receiptOcr: Boolean,
    @SerialName("balanceQueries") val balanceQueries: Boolean,
    @SerialName("paymentReminders") val paymentReminders: Boolean,
    @SerialName("maintenanceRequests") val maintenanceRequests: Boolean,
    @SerialName("humanHandoff") val humanHandoff: Boolean,
)

@Serializable
data class ChatbotConfigResponse(
    val id: String,
    @SerialName("organizacionId") val organizacionId: String,
    val activo: Boolean,
    @SerialName("connectionStatus") val connectionStatus: String,
    @SerialName("displayName") val displayName: String? = null,
    val language: String,
    val tone: String? = null,
    val greeting: String? = null,
    @SerialName("systemPrompt") val systemPrompt: String? = null,
    val faqs: List<FaqEntry>? = null,
    val policies: String? = null,
    @SerialName("senderPolicy") val senderPolicy: String,
    val allowlist: List<String>? = null,
    val capabilities: Capabilities,
    @SerialName("handoffKeywords") val handoffKeywords: List<String>? = null,
    @SerialName("historyLimit") val historyLimit: Int,
    @SerialName("retentionDays") val retentionDays: Int,
    @SerialName("createdAt") val createdAt: String,
    @SerialName("updatedAt") val updatedAt: String,
)

@Serializable
data class ChatbotConfigUpdateRequest(
    val activo: Boolean? = null,
    @SerialName("displayName") val displayName: String? = null,
    val language: String? = null,
    val tone: String? = null,
    val greeting: String? = null,
    @SerialName("systemPrompt") val systemPrompt: String? = null,
    val faqs: List<FaqEntry>? = null,
    val policies: String? = null,
    @SerialName("senderPolicy") val senderPolicy: String? = null,
    val allowlist: List<String>? = null,
    val capabilities: Capabilities? = null,
    @SerialName("handoffKeywords") val handoffKeywords: List<String>? = null,
    @SerialName("historyLimit") val historyLimit: Int? = null,
    @SerialName("retentionDays") val retentionDays: Int? = null,
)

@Serializable
data class ConnectionStatusResponse(
    val status: String,
    @SerialName("connectedPhone") val connectedPhone: String? = null,
    @SerialName("connectedAt") val connectedAt: String? = null,
)

@Serializable
data class TestChatHistoryEntry(
    val role: String,
    val content: String,
)

@Serializable
data class TestChatConfigOverride(
    @SerialName("displayName") val displayName: String? = null,
    val language: String? = null,
    val tone: String? = null,
    val greeting: String? = null,
    @SerialName("systemPrompt") val systemPrompt: String? = null,
    val faqs: List<FaqEntry>? = null,
    val policies: String? = null,
    val capabilities: Capabilities? = null,
    @SerialName("handoffKeywords") val handoffKeywords: List<String>? = null,
    @SerialName("historyLimit") val historyLimit: Int? = null,
)

@Serializable
data class TestChatRequest(
    val message: String,
    @SerialName("configOverride") val configOverride: TestChatConfigOverride? = null,
    val history: List<TestChatHistoryEntry> = emptyList(),
)

@Serializable
data class TestChatResponse(
    val reply: String,
    @SerialName("toolsInvoked") val toolsInvoked: List<String>,
)

@Serializable
data class ConversationListItem(
    @SerialName("senderPhone") val senderPhone: String,
    @SerialName("inquilinoId") val inquilinoId: String? = null,
    @SerialName("lastMessage") val lastMessage: String,
    @SerialName("lastMessageAt") val lastMessageAt: String,
    @SerialName("messageCount") val messageCount: Long,
)

@Serializable
data class ReceiptExtractionResponse(
    val id: String,
    @SerialName("organizacionId") val organizacionId: String,
    @SerialName("conversationId") val conversationId: String,
    @SerialName("inquilinoId") val inquilinoId: String? = null,
    @SerialName("contratoId") val contratoId: String? = null,
    val bank: String? = null,
    val amount: String,
    val currency: String,
    val date: String? = null,
    val reference: String? = null,
    @SerialName("senderName") val senderName: String? = null,
    val recipient: String? = null,
    val confidence: String,
    val status: String,
    @SerialName("confirmedBy") val confirmedBy: String? = null,
    @SerialName("createdAt") val createdAt: String,
    @SerialName("updatedAt") val updatedAt: String,
)

@Serializable
data class ReceiptConfirmRequest(
    @SerialName("contratoId") val contratoId: String? = null,
)

@Serializable
data class ReceiptRejectRequest(
    @SerialName("rejectionReason") val rejectionReason: String? = null,
)
