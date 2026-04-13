package com.propmanager.navigation

import com.propmanager.core.network.ApiErrorParser
import java.io.IOException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import retrofit2.Response

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
            HTTP_UNAUTHORIZED -> AppError.Unauthorized()
            HTTP_UNPROCESSABLE -> {
                val body = response.errorBody()?.string()
                val fieldErrors = parseFieldErrors(body)
                AppError.ValidationError(fieldErrors)
            }
            in HTTP_SERVER_ERROR_MIN..HTTP_SERVER_ERROR_MAX -> AppError.ServerError()
            else -> {
                val message = ApiErrorParser.extractMessage(response)
                AppError.GenericError(message)
            }
        }
    }

    private const val HTTP_UNAUTHORIZED = 401
    private const val HTTP_UNPROCESSABLE = 422
    private const val HTTP_SERVER_ERROR_MIN = 500
    private const val HTTP_SERVER_ERROR_MAX = 599

    fun handleException(throwable: Throwable): AppError =
        when (throwable) {
            is IOException -> AppError.NetworkError()
            else ->
                AppError.GenericError(
                    throwable.message ?: "Ha ocurrido un error. Intente nuevamente."
                )
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
