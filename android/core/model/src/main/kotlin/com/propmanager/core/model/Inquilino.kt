package com.propmanager.core.model

import java.time.Instant

data class Inquilino(
    val id: String,
    val nombre: String,
    val apellido: String,
    val email: String?,
    val telefono: String?,
    val cedula: String,
    val contactoEmergencia: String?,
    val notas: String?,
    val createdAt: Instant,
    val updatedAt: Instant,
    val isPendingSync: Boolean = false,
)
