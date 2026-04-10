package com.propmanager.core.database.mapper

import com.propmanager.core.database.entity.NotaMantenimientoEntity
import com.propmanager.core.model.NotaMantenimiento
import com.propmanager.core.model.dto.NotaDto
import java.time.Instant

fun NotaMantenimientoEntity.toDomain(): NotaMantenimiento =
    NotaMantenimiento(
        id = id,
        solicitudId = solicitudId,
        autorId = autorId,
        contenido = contenido,
        createdAt = Instant.ofEpochMilli(createdAt),
    )

fun NotaDto.toEntity(): NotaMantenimientoEntity =
    NotaMantenimientoEntity(
        id = id,
        solicitudId = solicitudId,
        autorId = autorId,
        contenido = contenido,
        createdAt = Instant.parse(createdAt).toEpochMilli(),
    )
