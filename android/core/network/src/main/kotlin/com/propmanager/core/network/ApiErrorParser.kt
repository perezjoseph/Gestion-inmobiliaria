package com.propmanager.core.network

import kotlinx.serialization.json.Json
import retrofit2.Response

object ApiErrorParser {
    private val json = Json { ignoreUnknownKeys = true }

    fun <T> extractMessage(response: Response<T>): String {
        val errorBody = response.errorBody()?.string() ?: return fallbackMessage(response.code())
        return try {
            val apiError = json.decodeFromString<ApiErrorBody>(errorBody)
            apiError.message
        } catch (_: Exception) {
            fallbackMessage(response.code())
        }
    }

    private fun fallbackMessage(code: Int): String =
        when (code) {
            400 -> "Solicitud inválida"
            401 -> "No autorizado"
            403 -> "Acceso denegado"
            404 -> "Recurso no encontrado"
            409 -> "Conflicto con datos existentes"
            422 -> "Datos de entrada inválidos"
            in 500..599 -> "Error del servidor"
            else -> "Error desconocido (código $code)"
        }
}

@kotlinx.serialization.Serializable
internal data class ApiErrorBody(
    val error: String,
    val message: String,
)
