package com.propmanager.core.data.repository

import com.propmanager.core.network.api.DocumentoDto
import com.propmanager.core.network.api.DocumentosApiService
import okhttp3.MultipartBody
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class DocumentosRepository @Inject constructor(
    private val apiService: DocumentosApiService
) {

    suspend fun fetchDocuments(entityType: String, entityId: String): Result<List<DocumentoDto>> = runCatching {
        apiService.list(entityType, entityId).body() ?: throw Exception("Empty response")
    }

    suspend fun uploadDocument(entityType: String, entityId: String, file: MultipartBody.Part): Result<DocumentoDto> = runCatching {
        apiService.upload(entityType, entityId, file).body() ?: throw Exception("Empty response")
    }
}
