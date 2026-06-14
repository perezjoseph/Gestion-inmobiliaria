package com.gestioninmobiliaria.data.local

import androidx.room.Entity
import androidx.room.PrimaryKey
import com.gestioninmobiliaria.data.model.Propiedad

@Entity(tableName = "propiedades")
data class PropiedadEntity(
    @PrimaryKey val id: String,
    val titulo: String,
    val descripcion: String?,
    val direccion: String,
    val ciudad: String,
    val provincia: String,
    val tipo_propiedad: String,
    val habitaciones: Int?,
    val banos: Int?,
    val area_m2: Double?,
    val precio: Double,
    val moneda: String,
    val estado: String,
    val total_unidades: Long?,
    val unidades_ocupadas: Long?,
    val tasa_ocupacion: Double?,
    val created_at: String,
    val updated_at: String,
)

fun PropiedadEntity.toModel() = Propiedad(
    id = id, titulo = titulo, descripcion = descripcion, direccion = direccion,
    ciudad = ciudad, provincia = provincia, tipo_propiedad = tipo_propiedad,
    habitaciones = habitaciones, banos = banos, area_m2 = area_m2,
    precio = precio, moneda = moneda, estado = estado,
    total_unidades = total_unidades, unidades_ocupadas = unidades_ocupadas,
    tasa_ocupacion = tasa_ocupacion, created_at = created_at, updated_at = updated_at,
)

fun Propiedad.toEntity() = PropiedadEntity(
    id = id, titulo = titulo, descripcion = descripcion, direccion = direccion,
    ciudad = ciudad, provincia = provincia, tipo_propiedad = tipo_propiedad,
    habitaciones = habitaciones, banos = banos, area_m2 = area_m2,
    precio = precio, moneda = moneda, estado = estado,
    total_unidades = total_unidades, unidades_ocupadas = unidades_ocupadas,
    tasa_ocupacion = tasa_ocupacion, created_at = created_at, updated_at = updated_at,
)
