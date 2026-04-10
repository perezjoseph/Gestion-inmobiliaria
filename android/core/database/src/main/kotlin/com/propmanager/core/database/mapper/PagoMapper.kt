package com.propmanager.core.database.mapper

import com.propmanager.core.database.entity.PagoEntity
import com.propmanager.core.model.Pago
import com.propmanager.core.model.dto.CreatePagoRequest
import com.propmanager.core.model.dto.PagoDto
import java.math.BigDecimal
import java.time.Instant
import java.time.LocalDate

fun PagoEntity.toDomain(): Pago =
    Pago(
        id = id,
        contratoId = contratoId,
        monto = BigDecimal(monto),
        moneda = moneda,
        fechaPago = fechaPago?.let { LocalDate.parse(it) },
        fechaVencimiento = LocalDate.parse(fechaVencimiento),
        metodoPago = metodoPago,
        estado = estado,
        notas = notas,
        createdAt = Instant.ofEpochMilli(createdAt),
        updatedAt = Instant.ofEpochMilli(updatedAt),
        isPendingSync = isPendingSync,
    )

fun PagoDto.toEntity(): PagoEntity =
    PagoEntity(
        id = id,
        contratoId = contratoId,
        monto = monto,
        moneda = moneda,
        fechaPago = fechaPago,
        fechaVencimiento = fechaVencimiento,
        metodoPago = metodoPago,
        estado = estado,
        notas = notas,
        createdAt = Instant.parse(createdAt).toEpochMilli(),
        updatedAt = Instant.parse(updatedAt).toEpochMilli(),
    )

fun Pago.toCreateRequest(): CreatePagoRequest =
    CreatePagoRequest(
        contratoId = contratoId,
        monto = monto.toPlainString(),
        moneda = moneda,
        fechaPago = fechaPago?.toString(),
        fechaVencimiento = fechaVencimiento.toString(),
        metodoPago = metodoPago,
        notas = notas,
    )
