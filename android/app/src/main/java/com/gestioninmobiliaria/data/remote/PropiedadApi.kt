package com.gestioninmobiliaria.data.remote

import com.gestioninmobiliaria.data.model.Propiedad
import retrofit2.http.GET

interface PropiedadApi {
    @GET("propiedades")
    suspend fun getPropiedades(): List<Propiedad>
}
