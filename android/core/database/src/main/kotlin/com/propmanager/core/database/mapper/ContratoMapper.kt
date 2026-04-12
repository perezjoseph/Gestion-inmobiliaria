package com.propmanager.core.database.mapper

import com.propmanager.core.database.entity.ContratoEntity
import com.propmanager.core.model.Contrato
import com.propmanager.core.model.dto.ContratoDto
import com.propmanager.core.model.dto.CreateContratoRequest
import java.math.BigDecimal
import java.time.Instant
import java.time.LocalDate

fun ContratoEntity.toDomain(): Contrato = Contrato(
    id = id,
    propiedadId = propiedadId,
    inquilinoId = inquilinoId,
    fechaInicio = LocalDate.parse(fechaInicio),
    fechaFin = LocalDate.parse(fechaFin),
    montoMensual = BigDecimal(montoMensual),
    deposito = deposito?.let { BigDecimal(it) },
    moneda = moneda,
    estado = estado,
    createdAt = Instant.ofEpochMilli(createdAt),
    updatedAt = Instant.ofEpochMilli(updatedAt),
    isPendingSync = isPendingSync
)

fun ContratoDto.toEntity(): ContratoEntity = ContratoEntity(
    id = id,
    propiedadId = propiedadId,
    inquilinoId = inquilinoId,
    fechaInicio = fechaInicio,
    fechaFin = fechaFin,
    montoMensual = montoMensual,
    deposito = deposito,
    moneda = moneda,
    estado = estado,
    createdAt = Instant.parse(createdAt).toEpochMilli(),
    updatedAt = Instant.parse(updatedAt).toEpochMilli()
)

fun Contrato.toCreateRequest(): CreateContratoRequest = CreateContratoRequest(
    propiedadId = propiedadId,
    inquilinoId = inquilinoId,
    fechaInicio = fechaInicio.toString(),
    fechaFin = fechaFin.toString(),
    montoMensual = montoMensual.toPlainString(),
    deposito = deposito?.toPlainString(),
    moneda = moneda
)
