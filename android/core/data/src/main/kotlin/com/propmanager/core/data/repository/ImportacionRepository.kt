package com.propmanager.core.data.repository

import com.propmanager.core.common.EmptyResponseException
import com.propmanager.core.network.api.ImportResultDto
import com.propmanager.core.network.api.ImportacionApiService
import javax.inject.Inject
import javax.inject.Singleton
import okhttp3.MultipartBody

@Singleton
class ImportacionRepository @Inject constructor(private val apiService: ImportacionApiService) {
    suspend fun importPropiedades(file: MultipartBody.Part): Result<ImportResultDto> = runCatching {
        apiService.importPropiedades(file).body()
            ?: throw EmptyResponseException("importacion/propiedades")
    }

    suspend fun importInquilinos(file: MultipartBody.Part): Result<ImportResultDto> = runCatching {
        apiService.importInquilinos(file).body()
            ?: throw EmptyResponseException("importacion/inquilinos")
    }

    suspend fun importGastos(file: MultipartBody.Part): Result<ImportResultDto> = runCatching {
        apiService.importGastos(file).body() ?: throw EmptyResponseException("importacion/gastos")
    }
}
