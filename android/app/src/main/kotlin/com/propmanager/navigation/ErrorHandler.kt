package com.propmanager.navigation

import com.propmanager.core.network.ApiErrorParser
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import retrofit2.Response
import java.io.IOException

sealed class AppError(val displayMessage: String) {
    class Unauthorized : AppError("Su sesión ha expirado. Inicie sesión nuevamente.")
    class ValidationError(val fieldErrors: Map<String, String>) : AppError("Error de validación")
    class ServerError : AppError("Error interno del servidor. Intente nuevamente más tarde.")
    class NetworkError : AppError("Sin conexión a internet. Los cambios se guardarán localmente.")
    class GenericError(message: String) : AppError(message)
}

object ErrorHandler {

    private val json = Json { ignoreUnknownKeys = true }

    fun <T> handleResponse(response: Response<T>): AppError? {
        if (response.isSuccessful) return null
        return when (response.code()) {
            401 -> AppError.Unauthorized()
            422 -> {
                val body = response.errorBody()?.string()
                val fieldErrors = parseFieldErrors(body)
                AppError.ValidationError(fieldErrors)
            }
            in 500..599 -> AppError.ServerError()
            else -> {
                val message = ApiErrorParser.extractMessage(response)
                AppError.GenericError(message)
            }
        }
    }

    fun handleException(throwable: Throwable): AppError {
        return when (throwable) {
            is IOException -> AppError.NetworkError()
            else -> AppError.GenericError(
                throwable.message ?: "Ha ocurrido un error. Intente nuevamente."
            )
        }
    }

    private fun parseFieldErrors(body: String?): Map<String, String> {
        if (body == null) return emptyMap()
        return try {
            val element = json.parseToJsonElement(body)
            val errors = element.jsonObject["errors"]?.jsonObject ?: return emptyMap()
            errors.mapValues { (_, v) -> v.jsonPrimitive.content }
        } catch (_: Exception) {
            emptyMap()
        }
    }
}
