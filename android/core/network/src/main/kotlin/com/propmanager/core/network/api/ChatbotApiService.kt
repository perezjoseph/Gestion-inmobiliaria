package com.propmanager.core.network.api

import com.propmanager.core.model.dto.ChatbotConfigResponse
import com.propmanager.core.model.dto.ChatbotConfigUpdateRequest
import com.propmanager.core.model.dto.ConnectionStatusResponse
import com.propmanager.core.model.dto.ConversationListItem
import com.propmanager.core.model.dto.ReceiptConfirmRequest
import com.propmanager.core.model.dto.ReceiptExtractionResponse
import com.propmanager.core.model.dto.ReceiptRejectRequest
import com.propmanager.core.model.dto.TestChatRequest
import com.propmanager.core.model.dto.TestChatResponse
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.PUT
import retrofit2.http.Path

interface ChatbotApiService {
    @GET("api/v1/chatbot/config")
    suspend fun getConfig(): Response<ChatbotConfigResponse>

    @PUT("api/v1/chatbot/config")
    suspend fun updateConfig(
        @Body request: ChatbotConfigUpdateRequest,
    ): Response<ChatbotConfigResponse>

    @GET("api/v1/chatbot/status")
    suspend fun getStatus(): Response<ConnectionStatusResponse>

    @POST("api/v1/chatbot/connect")
    suspend fun connect(): Response<ConnectionStatusResponse>

    @POST("api/v1/chatbot/disconnect")
    suspend fun disconnect(): Response<ConnectionStatusResponse>

    @POST("api/v1/chatbot/test")
    suspend fun testChat(
        @Body request: TestChatRequest,
    ): Response<TestChatResponse>

    @GET("api/v1/chatbot/conversations")
    suspend fun listConversations(): Response<List<ConversationListItem>>

    @GET("api/v1/chatbot/receipts/pending")
    suspend fun getPendingReceipts(): Response<List<ReceiptExtractionResponse>>

    @POST("api/v1/chatbot/receipts/{id}/confirm")
    suspend fun confirmReceipt(
        @Path("id") id: String,
        @Body request: ReceiptConfirmRequest,
    ): Response<ReceiptExtractionResponse>

    @POST("api/v1/chatbot/receipts/{id}/reject")
    suspend fun rejectReceipt(
        @Path("id") id: String,
        @Body request: ReceiptRejectRequest,
    ): Response<ReceiptExtractionResponse>
}
