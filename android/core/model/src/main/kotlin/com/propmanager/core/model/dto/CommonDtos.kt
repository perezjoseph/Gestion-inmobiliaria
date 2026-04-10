package com.propmanager.core.model.dto

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class PaginatedResponse<T>(
    val data: List<T>,
    val total: Long,
    val page: Long,
    @SerialName("perPage") val perPage: Long,
)

@Serializable data class ApiError(val error: String, val message: String)
