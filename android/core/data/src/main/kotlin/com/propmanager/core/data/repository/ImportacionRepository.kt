package com.propmanager.core.data.repository

import com.propmanager.core.network.api.ImportResultDto
import com.propmanager.core.network.api.ImportacionApiService
import okhttp3.MultipartBody
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ImportacionRepository @Inject constructor(
    private val apiService: ImportacionApiService
) {

    suspend fun importPropiedades(file: MultipartBody.Part): Result<ImportResultDto> = runCatching {
        apiService.importPropiedades(file).body() ?: throw Exception("Empty response")
    }

    suspend fun importInquilinos(file: MultipartBody.Part): Result<ImportResultDto> = runCatching {
        apiService.importInquilinos(file).body() ?: throw Exception("Empty response")
    }

    suspend fun importGastos(file: MultipartBody.Part): Result<ImportResultDto> = runCatching {
        apiService.importGastos(file).body() ?: throw Exception("Empty response")
    }
}
