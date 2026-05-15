package com.propmanager.core.network.api

import com.propmanager.core.model.dto.ActualizarPlantillaRequest
import com.propmanager.core.model.dto.CrearPlantillaRequest
import com.propmanager.core.model.dto.PlantillaResponse
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.PUT
import retrofit2.http.Path

interface PlantillasApiService {
    @GET("api/v1/plantillas")
    suspend fun getPlantillas(): Response<List<PlantillaResponse>>

    @POST("api/v1/plantillas")
    suspend fun createPlantilla(
        @Body request: CrearPlantillaRequest,
    ): Response<PlantillaResponse>

    @PUT("api/v1/plantillas/{id}")
    suspend fun updatePlantilla(
        @Path("id") id: String,
        @Body request: ActualizarPlantillaRequest,
    ): Response<PlantillaResponse>

    @DELETE("api/v1/plantillas/{id}")
    suspend fun deletePlantilla(
        @Path("id") id: String,
    ): Response<Unit>
}
