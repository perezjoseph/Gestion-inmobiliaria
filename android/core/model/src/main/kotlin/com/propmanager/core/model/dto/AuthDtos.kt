package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable data class LoginRequest(val email: String, val password: String)

@Serializable data class LoginResponse(val token: String, val user: UserDto)

@Serializable
data class UserDto(
    val id: String,
    val nombre: String,
    val email: String,
    val rol: String,
    val activo: Boolean,
    @SerialName("createdAt") val createdAt: String,
)
