package com.propmanager.core.common

import com.google.common.truth.Truth.assertThat
import com.propmanager.core.model.ValidationResult
import org.junit.jupiter.api.Test

class ValidatorsTest {
    private fun Map<String, ValidationResult>.assertAllValid() {
        values.forEach { assertThat(it).isInstanceOf(ValidationResult.Valid::class.java) }
    }

    private fun Map<String, ValidationResult>.assertInvalid(vararg keys: String) {
        keys.forEach { key ->
            assertThat(this[key]).isInstanceOf(ValidationResult.Invalid::class.java)
        }
    }

    // PropiedadValidator

    @Test
    fun `PropiedadValidator accepts valid input`() {
        PropiedadValidator
            .validateCreate(
                titulo = "Casa Centro",
                direccion = "Calle 1",
                ciudad = "Santo Domingo",
                provincia = "Distrito Nacional",
                tipoPropiedad = "casa",
                precio = "50000.00",
            ).assertAllValid()
    }

    @Test
    fun `PropiedadValidator rejects blank required fields`() {
        val result =
            PropiedadValidator.validateCreate(
                titulo = "",
                direccion = "  ",
                ciudad = "",
                provincia = "",
                tipoPropiedad = "",
                precio = "",
            )
        result.assertInvalid("titulo", "direccion", "ciudad", "provincia", "tipoPropiedad", "precio")
    }

    @Test
    fun `PropiedadValidator rejects non-positive precio`() {
        val result =
            PropiedadValidator.validateCreate(
                titulo = "Casa",
                direccion = "Calle 1",
                ciudad = "SD",
                provincia = "DN",
                tipoPropiedad = "casa",
                precio = "0",
            )
        result.assertInvalid("precio")
    }

    // InquilinoValidator

    @Test
    fun `InquilinoValidator accepts valid input`() {
        InquilinoValidator
            .validateCreate(
                nombre = "Juan",
                apellido = "Pérez",
                cedula = "001-1234567-8",
            ).assertAllValid()
    }

    @Test
    fun `InquilinoValidator rejects blank fields`() {
        InquilinoValidator
            .validateCreate(
                nombre = "",
                apellido = "",
                cedula = "  ",
            ).assertInvalid("nombre", "apellido", "cedula")
    }

    // ContratoValidator

    @Test
    fun `ContratoValidator accepts valid input`() {
        ContratoValidator
            .validateCreate(
                propiedadId = "abc-123",
                inquilinoId = "def-456",
                fechaInicio = "2025-01-01",
                fechaFin = "2026-01-01",
                montoMensual = "25000.00",
            ).assertAllValid()
    }

    @Test
    fun `ContratoValidator rejects blank required fields`() {
        ContratoValidator
            .validateCreate(
                propiedadId = "",
                inquilinoId = "",
                fechaInicio = "",
                fechaFin = "",
                montoMensual = "",
            ).assertInvalid("propiedadId", "inquilinoId", "fechaInicio", "fechaFin", "montoMensual")
    }

    @Test
    fun `ContratoValidator rejects fechaFin before fechaInicio`() {
        val result =
            ContratoValidator.validateCreate(
                propiedadId = "abc",
                inquilinoId = "def",
                fechaInicio = "2025-06-01",
                fechaFin = "2025-01-01",
                montoMensual = "10000",
            )
        result.assertInvalid("fechaFin")
    }

    @Test
    fun `ContratoValidator rejects fechaFin equal to fechaInicio`() {
        val result =
            ContratoValidator.validateCreate(
                propiedadId = "abc",
                inquilinoId = "def",
                fechaInicio = "2025-06-01",
                fechaFin = "2025-06-01",
                montoMensual = "10000",
            )
        result.assertInvalid("fechaFin")
    }

    // PagoValidator

    @Test
    fun `PagoValidator accepts valid input`() {
        PagoValidator
            .validateCreate(
                contratoId = "abc-123",
                monto = "25000.00",
                fechaVencimiento = "2025-07-01",
            ).assertAllValid()
    }

    @Test
    fun `PagoValidator rejects blank fields`() {
        PagoValidator
            .validateCreate(
                contratoId = "",
                monto = "",
                fechaVencimiento = "",
            ).assertInvalid("contratoId", "monto", "fechaVencimiento")
    }

    // GastoValidator

    @Test
    fun `GastoValidator accepts valid input`() {
        GastoValidator
            .validateCreate(
                propiedadId = "abc",
                categoria = "reparacion",
                descripcion = "Arreglo de tubería",
                monto = "5000.00",
                moneda = "DOP",
                fechaGasto = "2025-06-15",
            ).assertAllValid()
    }

    @Test
    fun `GastoValidator rejects blank fields`() {
        GastoValidator
            .validateCreate(
                propiedadId = "",
                categoria = "",
                descripcion = "",
                monto = "",
                moneda = "",
                fechaGasto = "",
            ).assertInvalid("propiedadId", "categoria", "descripcion", "monto", "moneda", "fechaGasto")
    }

    // SolicitudValidator

    @Test
    fun `SolicitudValidator accepts valid input`() {
        SolicitudValidator
            .validateCreate(
                propiedadId = "abc",
                titulo = "Fuga de agua",
            ).assertAllValid()
    }

    @Test
    fun `SolicitudValidator rejects blank fields`() {
        SolicitudValidator
            .validateCreate(
                propiedadId = "",
                titulo = "",
            ).assertInvalid("propiedadId", "titulo")
    }
}
