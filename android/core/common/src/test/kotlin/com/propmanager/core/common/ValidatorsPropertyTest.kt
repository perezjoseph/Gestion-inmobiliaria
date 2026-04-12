package com.propmanager.core.common

import com.propmanager.core.model.ValidationResult
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.types.shouldBeInstanceOf
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.element
import io.kotest.property.arbitrary.int
import io.kotest.property.arbitrary.string
import io.kotest.property.checkAll

/**
 * **Validates: Requirements 3.7, 4.6, 5.7, 6.7, 7.7, 8.8**
 *
 * Property 10: Entity form validation rejects blank required fields
 *
 * For any entity type (propiedad, inquilino, contrato, pago, gasto, solicitud)
 * and any form input where at least one required field is blank or whitespace-only,
 * the corresponding validator SHALL return an Invalid result for that field.
 * Conversely, for any form input where all required fields contain non-blank valid
 * values, the validator SHALL return Valid for all fields.
 */
class ValidatorsPropertyTest :
    FreeSpec({

        val blankArb: Arb<String> = Arb.element("", " ", "  ", "\t", "\n", "   \t\n  ")

        val nonBlankArb: Arb<String> =
            arbitrary {
                val base = Arb.string(minSize = 1, maxSize = 50).bind()
                if (base.isBlank()) "x$base" else base
            }

        val posDecimalArb: Arb<String> =
            arbitrary {
                val i = Arb.int(1..999999).bind()
                val d = Arb.int(0..99).bind()
                "$i.${d.toString().padStart(2, '0')}"
            }

        // Generates a pair of (fieldIndex, fields array) where the field at fieldIndex is blank
        fun blankFieldArb(
            fieldCount: Int,
            decimalIndices: Set<Int> = emptySet(),
        ): Arb<Pair<Int, Array<String>>> =
            arbitrary {
                val idx = Arb.int(0 until fieldCount).bind()
                val fields =
                    Array(fieldCount) { i ->
                        if (i == idx) {
                            blankArb.bind()
                        } else if (i in decimalIndices) {
                            posDecimalArb.bind()
                        } else {
                            nonBlankArb.bind()
                        }
                    }
                idx to fields
            }

        "Property 10: Entity form validation rejects blank required fields" -
            {

                val propFieldNames = arrayOf("titulo", "direccion", "ciudad", "provincia", "tipoPropiedad", "precio")

                "PropiedadValidator rejects any blank required field" {
                    checkAll(100, blankFieldArb(6, setOf(5))) { (blankIdx, fields) ->
                        val result =
                            PropiedadValidator.validateCreate(
                                titulo = fields[0],
                                direccion = fields[1],
                                ciudad = fields[2],
                                provincia = fields[3],
                                tipoPropiedad = fields[4],
                                precio = fields[5],
                            )
                        result[propFieldNames[blankIdx]].shouldBeInstanceOf<ValidationResult.Invalid>()
                    }
                }

                val inquFieldNames = arrayOf("nombre", "apellido", "cedula")

                "InquilinoValidator rejects any blank required field" {
                    checkAll(100, blankFieldArb(3)) { (blankIdx, fields) ->
                        val result =
                            InquilinoValidator.validateCreate(
                                nombre = fields[0],
                                apellido = fields[1],
                                cedula = fields[2],
                            )
                        result[inquFieldNames[blankIdx]].shouldBeInstanceOf<ValidationResult.Invalid>()
                    }
                }

                val contFieldNames = arrayOf("propiedadId", "inquilinoId", "fechaInicio", "fechaFin", "montoMensual")

                "ContratoValidator rejects any blank required field" {
                    // Use a custom arb so date fields get valid dates when not blank
                    val contBlankArb: Arb<Pair<Int, Array<String>>> =
                        arbitrary {
                            val idx = Arb.int(0..4).bind()
                            val fields =
                                Array(5) { i ->
                                    when {
                                        i == idx -> blankArb.bind()
                                        i == 2 -> "2025-01-01"
                                        i == 3 -> "2026-01-01"
                                        i == 4 -> posDecimalArb.bind()
                                        else -> nonBlankArb.bind()
                                    }
                                }
                            idx to fields
                        }
                    checkAll(100, contBlankArb) { (blankIdx, fields) ->
                        val result =
                            ContratoValidator.validateCreate(
                                propiedadId = fields[0],
                                inquilinoId = fields[1],
                                fechaInicio = fields[2],
                                fechaFin = fields[3],
                                montoMensual = fields[4],
                            )
                        result[contFieldNames[blankIdx]].shouldBeInstanceOf<ValidationResult.Invalid>()
                    }
                }

                val pagoFieldNames = arrayOf("contratoId", "monto", "fechaVencimiento")

                "PagoValidator rejects any blank required field" {
                    val pagoBlankArb: Arb<Pair<Int, Array<String>>> =
                        arbitrary {
                            val idx = Arb.int(0..2).bind()
                            val fields =
                                Array(3) { i ->
                                    when {
                                        i == idx -> blankArb.bind()
                                        i == 1 -> posDecimalArb.bind()
                                        i == 2 -> "2025-07-01"
                                        else -> nonBlankArb.bind()
                                    }
                                }
                            idx to fields
                        }
                    checkAll(100, pagoBlankArb) { (blankIdx, fields) ->
                        val result =
                            PagoValidator.validateCreate(
                                contratoId = fields[0],
                                monto = fields[1],
                                fechaVencimiento = fields[2],
                            )
                        result[pagoFieldNames[blankIdx]].shouldBeInstanceOf<ValidationResult.Invalid>()
                    }
                }

                val gastoFieldNames = arrayOf("propiedadId", "categoria", "descripcion", "monto", "moneda", "fechaGasto")

                "GastoValidator rejects any blank required field" {
                    val gastoBlankArb: Arb<Pair<Int, Array<String>>> =
                        arbitrary {
                            val idx = Arb.int(0..5).bind()
                            val fields =
                                Array(6) { i ->
                                    when {
                                        i == idx -> blankArb.bind()
                                        i == 3 -> posDecimalArb.bind()
                                        i == 5 -> "2025-06-15"
                                        else -> nonBlankArb.bind()
                                    }
                                }
                            idx to fields
                        }
                    checkAll(100, gastoBlankArb) { (blankIdx, fields) ->
                        val result =
                            GastoValidator.validateCreate(
                                propiedadId = fields[0],
                                categoria = fields[1],
                                descripcion = fields[2],
                                monto = fields[3],
                                moneda = fields[4],
                                fechaGasto = fields[5],
                            )
                        result[gastoFieldNames[blankIdx]].shouldBeInstanceOf<ValidationResult.Invalid>()
                    }
                }

                val solFieldNames = arrayOf("propiedadId", "titulo")

                "SolicitudValidator rejects any blank required field" {
                    checkAll(100, blankFieldArb(2)) { (blankIdx, fields) ->
                        val result =
                            SolicitudValidator.validateCreate(
                                propiedadId = fields[0],
                                titulo = fields[1],
                            )
                        result[solFieldNames[blankIdx]].shouldBeInstanceOf<ValidationResult.Invalid>()
                    }
                }

                "all validators return Valid when all required fields are non-blank" {
                    checkAll(100, nonBlankArb, nonBlankArb, posDecimalArb) { s1, s2, decimal ->
                        PropiedadValidator
                            .validateCreate(s1, s2, s1, s2, s1, decimal)
                            .values
                            .forEach { it.shouldBeInstanceOf<ValidationResult.Valid>() }

                        InquilinoValidator
                            .validateCreate(s1, s2, s1)
                            .values
                            .forEach { it.shouldBeInstanceOf<ValidationResult.Valid>() }

                        PagoValidator
                            .validateCreate(s1, decimal, "2025-07-01")
                            .values
                            .forEach { it.shouldBeInstanceOf<ValidationResult.Valid>() }

                        GastoValidator
                            .validateCreate(s1, s2, s1, decimal, s2, "2025-06-15")
                            .values
                            .forEach { it.shouldBeInstanceOf<ValidationResult.Valid>() }

                        SolicitudValidator
                            .validateCreate(s1, s2)
                            .values
                            .forEach { it.shouldBeInstanceOf<ValidationResult.Valid>() }

                        ContratoValidator
                            .validateCreate(s1, s2, "2025-01-01", "2026-01-01", decimal)
                            .values
                            .forEach { it.shouldBeInstanceOf<ValidationResult.Valid>() }
                    }
                }
            }
    })
