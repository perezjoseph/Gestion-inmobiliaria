package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class InquilinoDto(
    val id: String,
    val nombre: String,
    val apellido: String,
    val email: String? = null,
    val telefono: String? = null,
    val cedula: String,
    @SerialName("contactoEmergencia") val contactoEmergencia: String? = null,
    val notas: String? = null,
    @SerialName("createdAt") val createdAt: String,
    @SerialName("updatedAt") val updatedAt: String
)

@Serializable
data class CreateInquilinoRequest(
    val nombre: String,
    val apellido: String,
    val email: String? = null,
    val telefono: String? = null,
    val cedula: String,
    @SerialName("contactoEmergencia") val contactoEmergencia: String? = null,
    val notas: String? = null
)

@Serializable
data class UpdateInquilinoRequest(
    val nombre: String? = null,
    val apellido: String? = null,
    val email: String? = null,
    val telefono: String? = null,
    val cedula: String? = null,
    @SerialName("contactoEmergencia") val contactoEmergencia: String? = null,
    val notas: String? = null
)
