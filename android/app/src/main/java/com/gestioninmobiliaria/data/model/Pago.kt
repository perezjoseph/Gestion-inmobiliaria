package com.gestioninmobiliaria.data.model

import kotlinx.serialization.Serializable

@Serializable
enum class EstadoPago { pendiente, pagado, atrasado }

@Serializable
data class Pago(
    val id: Int,
    val contrato_id: Int,
    val monto: Double,
    val moneda: String,
    val fecha_vencimiento: String,
    val fecha_pago: String? = null,
    val estado: EstadoPago,
    val metodo_pago: String? = null,
)
