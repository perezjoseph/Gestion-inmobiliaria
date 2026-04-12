package com.propmanager.core.database.mapper

import com.propmanager.core.database.entity.SolicitudMantenimientoEntity
import com.propmanager.core.model.SolicitudMantenimiento
import com.propmanager.core.model.dto.CreateSolicitudRequest
import com.propmanager.core.model.dto.SolicitudDto
import java.math.BigDecimal
import java.time.Instant

fun SolicitudMantenimientoEntity.toDomain(): SolicitudMantenimiento =
    SolicitudMantenimiento(
        id = id,
        propiedadId = propiedadId,
        unidadId = unidadId,
        inquilinoId = inquilinoId,
        titulo = titulo,
        descripcion = descripcion,
        estado = estado,
        prioridad = prioridad,
        nombreProveedor = nombreProveedor,
        telefonoProveedor = telefonoProveedor,
        emailProveedor = emailProveedor,
        costoMonto = costoMonto?.let { BigDecimal(it) },
        costoMoneda = costoMoneda,
        fechaInicio = fechaInicio?.let { Instant.ofEpochMilli(it) },
        fechaFin = fechaFin?.let { Instant.ofEpochMilli(it) },
        createdAt = Instant.ofEpochMilli(createdAt),
        updatedAt = Instant.ofEpochMilli(updatedAt),
        isPendingSync = isPendingSync,
    )

fun SolicitudDto.toEntity(): SolicitudMantenimientoEntity =
    SolicitudMantenimientoEntity(
        id = id,
        propiedadId = propiedadId,
        unidadId = unidadId,
        inquilinoId = inquilinoId,
        titulo = titulo,
        descripcion = descripcion,
        estado = estado,
        prioridad = prioridad,
        nombreProveedor = nombreProveedor,
        telefonoProveedor = telefonoProveedor,
        emailProveedor = emailProveedor,
        costoMonto = costoMonto,
        costoMoneda = costoMoneda,
        fechaInicio = fechaInicio?.let { Instant.parse(it).toEpochMilli() },
        fechaFin = fechaFin?.let { Instant.parse(it).toEpochMilli() },
        createdAt = Instant.parse(createdAt).toEpochMilli(),
        updatedAt = Instant.parse(updatedAt).toEpochMilli(),
    )

fun SolicitudMantenimiento.toCreateRequest(): CreateSolicitudRequest =
    CreateSolicitudRequest(
        propiedadId = propiedadId,
        unidadId = unidadId,
        inquilinoId = inquilinoId,
        titulo = titulo,
        descripcion = descripcion,
        prioridad = prioridad,
        nombreProveedor = nombreProveedor,
        telefonoProveedor = telefonoProveedor,
        emailProveedor = emailProveedor,
        costoMonto = costoMonto?.toPlainString(),
        costoMoneda = costoMoneda,
    )
