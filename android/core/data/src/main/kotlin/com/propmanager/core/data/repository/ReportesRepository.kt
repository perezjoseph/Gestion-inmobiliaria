package com.propmanager.core.data.repository

import com.propmanager.core.common.EmptyResponseException
import com.propmanager.core.model.dto.HistorialPagoReporte
import com.propmanager.core.model.dto.IngresoReporteSummary
import com.propmanager.core.model.dto.OcupacionTendencia
import com.propmanager.core.model.dto.RentabilidadReporteSummary
import com.propmanager.core.network.api.ReportesApiService
import javax.inject.Inject
import javax.inject.Singleton
import okhttp3.ResponseBody

@Singleton
class ReportesRepository @Inject constructor(private val apiService: ReportesApiService) {
    suspend fun fetchIngresos(
        params: Map<String, String> = emptyMap()
    ): Result<IngresoReporteSummary> = runCatching {
        apiService.ingresos(params).body() ?: throw EmptyResponseException("reportes/ingresos")
    }

    suspend fun fetchRentabilidad(
        params: Map<String, String> = emptyMap()
    ): Result<RentabilidadReporteSummary> = runCatching {
        apiService.rentabilidad(params).body()
            ?: throw EmptyResponseException("reportes/rentabilidad")
    }

    suspend fun fetchHistorialPagos(
        params: Map<String, String> = emptyMap()
    ): Result<List<HistorialPagoReporte>> = runCatching {
        apiService.historialPagos(params).body()
            ?: throw EmptyResponseException("reportes/historial-pagos")
    }

    suspend fun fetchOcupacionTendencia(
        params: Map<String, String> = emptyMap()
    ): Result<List<OcupacionTendencia>> = runCatching {
        apiService.ocupacionTendencia(params).body()
            ?: throw EmptyResponseException("reportes/ocupacion")
    }

    suspend fun downloadIngresosPdf(
        params: Map<String, String> = emptyMap()
    ): Result<ResponseBody> = runCatching {
        apiService.ingresosPdf(params).body()
            ?: throw EmptyResponseException("reportes/ingresos/pdf")
    }

    suspend fun downloadIngresosXlsx(
        params: Map<String, String> = emptyMap()
    ): Result<ResponseBody> = runCatching {
        apiService.ingresosXlsx(params).body()
            ?: throw EmptyResponseException("reportes/ingresos/xlsx")
    }

    suspend fun downloadRentabilidadPdf(
        params: Map<String, String> = emptyMap()
    ): Result<ResponseBody> = runCatching {
        apiService.rentabilidadPdf(params).body()
            ?: throw EmptyResponseException("reportes/rentabilidad/pdf")
    }

    suspend fun downloadRentabilidadXlsx(
        params: Map<String, String> = emptyMap()
    ): Result<ResponseBody> = runCatching {
        apiService.rentabilidadXlsx(params).body()
            ?: throw EmptyResponseException("reportes/rentabilidad/xlsx")
    }
}
