package com.propmanager.core.common

import com.propmanager.core.model.ValidationResult
import java.math.BigDecimal

private fun requireNotBlank(value: String, message: String): ValidationResult =
    if (value.isBlank()) ValidationResult.Invalid(message) else ValidationResult.Valid

private fun requirePositiveDecimal(value: String, message: String): ValidationResult {
    val decimal = value.toBigDecimalOrNull()
    return if (decimal == null || decimal <= BigDecimal.ZERO) {
        ValidationResult.Invalid(message)
    } else {
        ValidationResult.Valid
    }
}

object PropiedadValidator {
    fun validateCreate(
        titulo: String,
        direccion: String,
        ciudad: String,
        provincia: String,
        tipoPropiedad: String,
        precio: String,
    ): Map<String, ValidationResult> =
        mapOf(
            "titulo" to requireNotBlank(titulo, "El título es requerido"),
            "direccion" to requireNotBlank(direccion, "La dirección es requerida"),
            "ciudad" to requireNotBlank(ciudad, "La ciudad es requerida"),
            "provincia" to requireNotBlank(provincia, "La provincia es requerida"),
            "tipoPropiedad" to requireNotBlank(tipoPropiedad, "El tipo de propiedad es requerido"),
            "precio" to requirePositiveDecimal(precio, "El precio debe ser mayor a cero"),
        )
}

object InquilinoValidator {
    fun validateCreate(
        nombre: String,
        apellido: String,
        cedula: String,
    ): Map<String, ValidationResult> =
        mapOf(
            "nombre" to requireNotBlank(nombre, "El nombre es requerido"),
            "apellido" to requireNotBlank(apellido, "El apellido es requerido"),
            "cedula" to requireNotBlank(cedula, "La cédula es requerida"),
        )
}

private const val MSG_PROPIEDAD_REQUERIDA = "La propiedad es requerida"

object ContratoValidator {
    fun validateCreate(
        propiedadId: String,
        inquilinoId: String,
        fechaInicio: String,
        fechaFin: String,
        montoMensual: String,
    ): Map<String, ValidationResult> {
        val results =
            mutableMapOf<String, ValidationResult>(
                "propiedadId" to requireNotBlank(propiedadId, MSG_PROPIEDAD_REQUERIDA),
                "inquilinoId" to requireNotBlank(inquilinoId, "El inquilino es requerido"),
                "fechaInicio" to requireNotBlank(fechaInicio, "La fecha de inicio es requerida"),
                "fechaFin" to requireNotBlank(fechaFin, "La fecha de fin es requerida"),
                "montoMensual" to
                    requirePositiveDecimal(montoMensual, "El monto mensual debe ser mayor a cero"),
            )

        if (fechaInicio.isNotBlank() && fechaFin.isNotBlank()) {
            try {
                val inicio = DateFormatter.fromApi(fechaInicio)
                val fin = DateFormatter.fromApi(fechaFin)
                if (!fin.isAfter(inicio)) {
                    results["fechaFin"] =
                        ValidationResult.Invalid(
                            "La fecha de fin debe ser posterior a la fecha de inicio"
                        )
                }
            } catch (_: Exception) {
                // Date parsing errors are handled by the blank checks above
            }
        }

        return results
    }
}

object PagoValidator {
    fun validateCreate(
        contratoId: String,
        monto: String,
        fechaVencimiento: String,
    ): Map<String, ValidationResult> =
        mapOf(
            "contratoId" to requireNotBlank(contratoId, "El contrato es requerido"),
            "monto" to requirePositiveDecimal(monto, "El monto debe ser mayor a cero"),
            "fechaVencimiento" to
                requireNotBlank(fechaVencimiento, "La fecha de vencimiento es requerida"),
        )
}

object GastoValidator {
    fun validateCreate(
        propiedadId: String,
        categoria: String,
        descripcion: String,
        monto: String,
        moneda: String,
        fechaGasto: String,
    ): Map<String, ValidationResult> =
        mapOf(
            "propiedadId" to requireNotBlank(propiedadId, MSG_PROPIEDAD_REQUERIDA),
            "categoria" to requireNotBlank(categoria, "La categoría es requerida"),
            "descripcion" to requireNotBlank(descripcion, "La descripción es requerida"),
            "monto" to requirePositiveDecimal(monto, "El monto debe ser mayor a cero"),
            "moneda" to requireNotBlank(moneda, "La moneda es requerida"),
            "fechaGasto" to requireNotBlank(fechaGasto, "La fecha del gasto es requerida"),
        )
}

object SolicitudValidator {
    fun validateCreate(propiedadId: String, titulo: String): Map<String, ValidationResult> =
        mapOf(
            "propiedadId" to requireNotBlank(propiedadId, MSG_PROPIEDAD_REQUERIDA),
            "titulo" to requireNotBlank(titulo, "El título es requerido"),
        )
}
