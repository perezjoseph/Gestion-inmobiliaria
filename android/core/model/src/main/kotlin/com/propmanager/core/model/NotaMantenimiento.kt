package com.propmanager.core.model

import java.time.Instant

data class NotaMantenimiento(
    val id: String,
    val solicitudId: String,
    val autorId: String,
    val contenido: String,
    val createdAt: Instant
)
