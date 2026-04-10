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
            HTTP_BAD_REQUEST -> "Solicitud inválida"
            HTTP_UNAUTHORIZED -> "No autorizado"
            HTTP_FORBIDDEN -> "Acceso denegado"
            HTTP_NOT_FOUND -> "Recurso no encontrado"
            HTTP_CONFLICT -> "Conflicto con datos existentes"
            HTTP_UNPROCESSABLE -> "Datos de entrada inválidos"
            in HTTP_SERVER_ERROR_MIN..HTTP_SERVER_ERROR_MAX -> "Error del servidor"
            else -> "Error desconocido (código $code)"
        }

    private const val HTTP_BAD_REQUEST = 400
    private const val HTTP_UNAUTHORIZED = 401
    private const val HTTP_FORBIDDEN = 403
    private const val HTTP_NOT_FOUND = 404
    private const val HTTP_CONFLICT = 409
    private const val HTTP_UNPROCESSABLE = 422
    private const val HTTP_SERVER_ERROR_MIN = 500
    private const val HTTP_SERVER_ERROR_MAX = 599
}

@kotlinx.serialization.Serializable
internal data class ApiErrorBody(val error: String, val message: String)
