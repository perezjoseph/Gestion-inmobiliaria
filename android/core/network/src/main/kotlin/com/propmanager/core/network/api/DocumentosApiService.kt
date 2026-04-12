package com.propmanager.core.network.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import okhttp3.MultipartBody
import retrofit2.Response
import retrofit2.http.GET
import retrofit2.http.Multipart
import retrofit2.http.POST
import retrofit2.http.Part
import retrofit2.http.Path

@Serializable
data class DocumentoDto(
    val id: String,
    @SerialName("entityType") val entityType: String,
    @SerialName("entityId") val entityId: String,
    val filename: String,
    @SerialName("filePath") val filePath: String,
    @SerialName("mimeType") val mimeType: String,
    @SerialName("fileSize") val fileSize: Long,
    @SerialName("uploadedBy") val uploadedBy: String,
    @SerialName("createdAt") val createdAt: String
)

interface DocumentosApiService {

    @GET("api/documentos/{entityType}/{entityId}")
    suspend fun list(
        @Path("entityType") entityType: String,
        @Path("entityId") entityId: String
    ): Response<List<DocumentoDto>>

    @Multipart
    @POST("api/documentos/{entityType}/{entityId}")
    suspend fun upload(
        @Path("entityType") entityType: String,
        @Path("entityId") entityId: String,
        @Part file: MultipartBody.Part
    ): Response<DocumentoDto>
}
