package com.gestioninmobiliaria.data.model

import kotlinx.serialization.Serializable

@Serializable
data class Propiedad(
    val id: String,
    val titulo: String,
    val descripcion: String? = null,
    val direccion: String,
    val ciudad: String,
    val provincia: String,
    val tipo_propiedad: String,
    val habitaciones: Int? = null,
    val banos: Int? = null,
    val area_m2: Double? = null,
    val precio: Double,
    val moneda: String,
    val estado: String,
    val total_unidades: Long? = null,
    val unidades_ocupadas: Long? = null,
    val tasa_ocupacion: Double? = null,
    val created_at: String,
    val updated_at: String,
)
