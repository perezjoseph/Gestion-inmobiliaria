package com.propmanager.core.database.mapper

import com.propmanager.core.database.entity.GastoEntity
import com.propmanager.core.model.Gasto
import com.propmanager.core.model.dto.CreateGastoRequest
import com.propmanager.core.model.dto.GastoDto
import java.math.BigDecimal
import java.time.Instant
import java.time.LocalDate

fun GastoEntity.toDomain(): Gasto =
    Gasto(
        id = id,
        propiedadId = propiedadId,
        unidadId = unidadId,
        categoria = categoria,
        descripcion = descripcion,
        monto = BigDecimal(monto),
        moneda = moneda,
        fechaGasto = LocalDate.parse(fechaGasto),
        estado = estado,
        proveedor = proveedor,
        numeroFactura = numeroFactura,
        notas = notas,
        createdAt = Instant.ofEpochMilli(createdAt),
        updatedAt = Instant.ofEpochMilli(updatedAt),
        isPendingSync = isPendingSync,
    )

fun GastoDto.toEntity(): GastoEntity =
    GastoEntity(
        id = id,
        propiedadId = propiedadId,
        unidadId = unidadId,
        categoria = categoria,
        descripcion = descripcion,
        monto = monto,
        moneda = moneda,
        fechaGasto = fechaGasto,
        estado = estado,
        proveedor = proveedor,
        numeroFactura = numeroFactura,
        notas = notas,
        createdAt = Instant.parse(createdAt).toEpochMilli(),
        updatedAt = Instant.parse(updatedAt).toEpochMilli(),
    )

fun Gasto.toCreateRequest(): CreateGastoRequest =
    CreateGastoRequest(
        propiedadId = propiedadId,
        unidadId = unidadId,
        categoria = categoria,
        descripcion = descripcion,
        monto = monto.toPlainString(),
        moneda = moneda,
        fechaGasto = fechaGasto.toString(),
        proveedor = proveedor,
        numeroFactura = numeroFactura,
        notas = notas,
    )
