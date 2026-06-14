package com.gestioninmobiliaria.data.repository

import com.gestioninmobiliaria.data.model.Pago
import com.gestioninmobiliaria.data.remote.CreatePagoRequest
import com.gestioninmobiliaria.data.remote.PagoApi
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class PagoRepository @Inject constructor(private val api: PagoApi) {
    suspend fun getPagos(): List<Pago> = api.getPagos()

    suspend fun createPago(
        contratoId: Int,
        monto: Double,
        moneda: String,
        metodoPago: String,
        fechaPago: String,
    ): Pago = api.createPago(
        CreatePagoRequest(
            contrato_id = contratoId,
            monto = monto,
            moneda = moneda,
            metodo_pago = metodoPago,
            fecha_pago = fechaPago,
        )
    )
}
