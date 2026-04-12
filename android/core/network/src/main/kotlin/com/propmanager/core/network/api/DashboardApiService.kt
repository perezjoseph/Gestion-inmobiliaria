package com.propmanager.core.network.api

import com.propmanager.core.model.dto.ContratoCalendario
import com.propmanager.core.model.dto.DashboardStats
import com.propmanager.core.model.dto.GastosComparacion
import com.propmanager.core.model.dto.IngresosComparacion
import com.propmanager.core.model.dto.OcupacionTendencia
import com.propmanager.core.model.dto.PagoProximo
import retrofit2.Response
import retrofit2.http.GET

interface DashboardApiService {

    @GET("api/dashboard/stats")
    suspend fun stats(): Response<DashboardStats>

    @GET("api/dashboard/pagos-proximos")
    suspend fun pagosProximos(): Response<List<PagoProximo>>

    @GET("api/dashboard/contratos-calendario")
    suspend fun contratosCalendario(): Response<List<ContratoCalendario>>

    @GET("api/dashboard/ocupacion-tendencia")
    suspend fun ocupacionTendencia(): Response<List<OcupacionTendencia>>

    @GET("api/dashboard/ingresos-comparacion")
    suspend fun ingresosComparacion(): Response<IngresosComparacion>

    @GET("api/dashboard/gastos-comparacion")
    suspend fun gastosComparacion(): Response<GastosComparacion>
}
