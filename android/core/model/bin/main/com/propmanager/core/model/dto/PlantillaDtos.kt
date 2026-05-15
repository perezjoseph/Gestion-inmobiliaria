package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

@Serializable
data class PlantillaResponse(
    @SerialName("id") val id: String,
    @SerialName("nombre") val nombre: String,
    @SerialName("tipo_documento") val tipoDocumento: String,
    @SerialName("entity_type") val entityType: String,
    @SerialName("contenido") val contenido: JsonElement,
)

@Serializable
data class CrearPlantillaRequest(
    @SerialName("nombre") val nombre: String,
    @SerialName("tipo_documento") val tipoDocumento: String,
    @SerialName("entity_type") val entityType: String,
    @SerialName("contenido") val contenido: JsonElement,
)

@Serializable
data class ActualizarPlantillaRequest(
    @SerialName("nombre") val nombre: String? = null,
    @SerialName("tipo_documento") val tipoDocumento: String? = null,
    @SerialName("entity_type") val entityType: String? = null,
    @SerialName("contenido") val contenido: JsonElement? = null,
)
