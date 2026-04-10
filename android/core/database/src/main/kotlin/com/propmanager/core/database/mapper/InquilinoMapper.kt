package com.propmanager.core.database.mapper

import com.propmanager.core.database.entity.InquilinoEntity
import com.propmanager.core.model.Inquilino
import com.propmanager.core.model.dto.CreateInquilinoRequest
import com.propmanager.core.model.dto.InquilinoDto
import java.time.Instant

fun InquilinoEntity.toDomain(): Inquilino =
    Inquilino(
        id = id,
        nombre = nombre,
        apellido = apellido,
        email = email,
        telefono = telefono,
        cedula = cedula,
        contactoEmergencia = contactoEmergencia,
        notas = notas,
        createdAt = Instant.ofEpochMilli(createdAt),
        updatedAt = Instant.ofEpochMilli(updatedAt),
        isPendingSync = isPendingSync,
    )

fun InquilinoDto.toEntity(): InquilinoEntity =
    InquilinoEntity(
        id = id,
        nombre = nombre,
        apellido = apellido,
        email = email,
        telefono = telefono,
        cedula = cedula,
        contactoEmergencia = contactoEmergencia,
        notas = notas,
        createdAt = Instant.parse(createdAt).toEpochMilli(),
        updatedAt = Instant.parse(updatedAt).toEpochMilli(),
    )

fun Inquilino.toCreateRequest(): CreateInquilinoRequest =
    CreateInquilinoRequest(
        nombre = nombre,
        apellido = apellido,
        email = email,
        telefono = telefono,
        cedula = cedula,
        contactoEmergencia = contactoEmergencia,
        notas = notas,
    )
