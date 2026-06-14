package com.gestioninmobiliaria.data.repository

import com.gestioninmobiliaria.data.model.Pago
import com.gestioninmobiliaria.data.remote.PagoApi
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class PagoRepository @Inject constructor(private val api: PagoApi) {
    suspend fun getPagos(): List<Pago> = api.getPagos()
}
