package com.propmanager.core.database.mapper

import com.propmanager.core.database.entity.PropiedadEntity
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.dto.CreatePropiedadRequest
import com.propmanager.core.model.dto.PropiedadDto
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonPrimitive
import java.math.BigDecimal
import java.time.Instant

fun PropiedadEntity.toDomain(): Propiedad =
    Propiedad(
        id = id,
        titulo = titulo,
        descripcion = descripcion,
        direccion = direccion,
        ciudad = ciudad,
        provincia = provincia,
        tipoPropiedad = tipoPropiedad,
        habitaciones = habitaciones,
        banos = banos,
        areaM2 = areaM2?.let { BigDecimal(it) },
        precio = BigDecimal(precio),
        moneda = moneda,
        estado = estado,
        imagenes = imagenes?.let { parseImagenesJson(it) } ?: emptyList(),
        createdAt = Instant.ofEpochMilli(createdAt),
        updatedAt = Instant.ofEpochMilli(updatedAt),
        isPendingSync = isPendingSync,
    )

fun PropiedadDto.toEntity(): PropiedadEntity =
    PropiedadEntity(
        id = id,
        titulo = titulo,
        descripcion = descripcion,
        direccion = direccion,
        ciudad = ciudad,
        provincia = provincia,
        tipoPropiedad = tipoPropiedad,
        habitaciones = habitaciones,
        banos = banos,
        areaM2 = areaM2,
        precio = precio,
        moneda = moneda,
        estado = estado,
        imagenes = imagenes?.toString(),
        createdAt = Instant.parse(createdAt).toEpochMilli(),
        updatedAt = Instant.parse(updatedAt).toEpochMilli(),
    )

fun Propiedad.toCreateRequest(): CreatePropiedadRequest =
    CreatePropiedadRequest(
        titulo = titulo,
        descripcion = descripcion,
        direccion = direccion,
        ciudad = ciudad,
        provincia = provincia,
        tipoPropiedad = tipoPropiedad,
        habitaciones = habitaciones,
        banos = banos,
        areaM2 = areaM2?.toPlainString(),
        precio = precio.toPlainString(),
        moneda = moneda,
        estado = estado,
        imagenes =
            if (imagenes.isNotEmpty()) {
                JsonArray(imagenes.map { JsonPrimitive(it) })
            } else {
                null
            },
    )

private fun parseImagenesJson(json: String): List<String> =
    try {
        kotlinx.serialization.json.Json
            .parseToJsonElement(json)
            .jsonArray
            .map { it.jsonPrimitive.content }
    } catch (_: Exception) {
        emptyList()
    }
