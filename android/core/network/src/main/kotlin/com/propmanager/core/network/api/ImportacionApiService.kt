package com.propmanager.core.network.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import okhttp3.MultipartBody
import retrofit2.Response
import retrofit2.http.Multipart
import retrofit2.http.POST
import retrofit2.http.Part

@Serializable
data class ImportResultDto(
    @SerialName("totalFilas") val totalFilas: Int,
    val exitosos: Int,
    val fallidos: List<ImportErrorDto>,
)

@Serializable
data class ImportErrorDto(
    val fila: Int,
    val error: String,
)

interface ImportacionApiService {
    @Multipart
    @POST("api/importar/propiedades")
    suspend fun importPropiedades(
        @Part file: MultipartBody.Part,
    ): Response<ImportResultDto>

    @Multipart
    @POST("api/importar/inquilinos")
    suspend fun importInquilinos(
        @Part file: MultipartBody.Part,
    ): Response<ImportResultDto>

    @Multipart
    @POST("api/importar/gastos")
    suspend fun importGastos(
        @Part file: MultipartBody.Part,
    ): Response<ImportResultDto>
}
