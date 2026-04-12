package com.propmanager.core.model

sealed class ValidationResult {
    data object Valid : ValidationResult()

    data class Invalid(
        val message: String,
    ) : ValidationResult()
}
