package com.propmanager.core.data.repository

import com.propmanager.core.model.dto.HistorialPagoReporte
import com.propmanager.core.model.dto.IngresoReporteSummary
import com.propmanager.core.model.dto.OcupacionTendencia
import com.propmanager.core.model.dto.RentabilidadReporteSummary
import com.propmanager.core.network.api.ReportesApiService
import okhttp3.ResponseBody
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ReportesRepository
    @Inject
    constructor(
        private val apiService: ReportesApiService,
    ) {
        suspend fun fetchIngresos(params: Map<String, String> = emptyMap()): Result<IngresoReporteSummary> =
            runCatching {
                apiService.ingresos(params).body() ?: throw Exception("Empty response")
            }

        suspend fun fetchRentabilidad(params: Map<String, String> = emptyMap()): Result<RentabilidadReporteSummary> =
            runCatching {
                apiService.rentabilidad(params).body() ?: throw Exception("Empty response")
            }

        suspend fun fetchHistorialPagos(params: Map<String, String> = emptyMap()): Result<List<HistorialPagoReporte>> =
            runCatching {
                apiService.historialPagos(params).body() ?: throw Exception("Empty response")
            }

        suspend fun fetchOcupacionTendencia(params: Map<String, String> = emptyMap()): Result<List<OcupacionTendencia>> =
            runCatching {
                apiService.ocupacionTendencia(params).body() ?: throw Exception("Empty response")
            }

        suspend fun downloadIngresosPdf(params: Map<String, String> = emptyMap()): Result<ResponseBody> =
            runCatching {
                apiService.ingresosPdf(params).body() ?: throw Exception("Empty response")
            }

        suspend fun downloadIngresosXlsx(params: Map<String, String> = emptyMap()): Result<ResponseBody> =
            runCatching {
                apiService.ingresosXlsx(params).body() ?: throw Exception("Empty response")
            }

        suspend fun downloadRentabilidadPdf(params: Map<String, String> = emptyMap()): Result<ResponseBody> =
            runCatching {
                apiService.rentabilidadPdf(params).body() ?: throw Exception("Empty response")
            }

        suspend fun downloadRentabilidadXlsx(params: Map<String, String> = emptyMap()): Result<ResponseBody> =
            runCatching {
                apiService.rentabilidadXlsx(params).body() ?: throw Exception("Empty response")
            }
    }
