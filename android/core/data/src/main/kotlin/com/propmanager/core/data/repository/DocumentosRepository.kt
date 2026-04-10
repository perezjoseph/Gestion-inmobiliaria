package com.propmanager.core.data.repository

import com.propmanager.core.common.EmptyResponseException
import com.propmanager.core.network.api.DocumentoDto
import com.propmanager.core.network.api.DocumentosApiService
import javax.inject.Inject
import javax.inject.Singleton
import okhttp3.MultipartBody

@Singleton
class DocumentosRepository @Inject constructor(private val apiService: DocumentosApiService) {
    suspend fun fetchDocuments(entityType: String, entityId: String): Result<List<DocumentoDto>> =
        runCatching {
            apiService.list(entityType, entityId).body()
                ?: throw EmptyResponseException("documentos")
        }

    suspend fun uploadDocument(
        entityType: String,
        entityId: String,
        file: MultipartBody.Part,
    ): Result<DocumentoDto> = runCatching {
        apiService.upload(entityType, entityId, file).body()
            ?: throw EmptyResponseException("documentos/upload")
    }
}
