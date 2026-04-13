package com.propmanager.core.network.api

import com.propmanager.core.model.dto.HistorialPagoReporte
import com.propmanager.core.model.dto.IngresoReporteSummary
import com.propmanager.core.model.dto.OcupacionTendencia
import com.propmanager.core.model.dto.RentabilidadReporteSummary
import okhttp3.ResponseBody
import retrofit2.Response
import retrofit2.http.GET
import retrofit2.http.QueryMap

interface ReportesApiService {
    @GET("api/reportes/ingresos")
    suspend fun ingresos(
        @QueryMap params: Map<String, String> = emptyMap()
    ): Response<IngresoReporteSummary>

    @GET("api/reportes/rentabilidad")
    suspend fun rentabilidad(
        @QueryMap params: Map<String, String> = emptyMap()
    ): Response<RentabilidadReporteSummary>

    @GET("api/reportes/historial-pagos")
    suspend fun historialPagos(
        @QueryMap params: Map<String, String> = emptyMap()
    ): Response<List<HistorialPagoReporte>>

    @GET("api/reportes/ocupacion/tendencia")
    suspend fun ocupacionTendencia(
        @QueryMap params: Map<String, String> = emptyMap()
    ): Response<List<OcupacionTendencia>>

    @GET("api/reportes/ingresos/pdf")
    suspend fun ingresosPdf(
        @QueryMap params: Map<String, String> = emptyMap()
    ): Response<ResponseBody>

    @GET("api/reportes/ingresos/xlsx")
    suspend fun ingresosXlsx(
        @QueryMap params: Map<String, String> = emptyMap()
    ): Response<ResponseBody>

    @GET("api/reportes/rentabilidad/pdf")
    suspend fun rentabilidadPdf(
        @QueryMap params: Map<String, String> = emptyMap()
    ): Response<ResponseBody>

    @GET("api/reportes/rentabilidad/xlsx")
    suspend fun rentabilidadXlsx(
        @QueryMap params: Map<String, String> = emptyMap()
    ): Response<ResponseBody>
}
