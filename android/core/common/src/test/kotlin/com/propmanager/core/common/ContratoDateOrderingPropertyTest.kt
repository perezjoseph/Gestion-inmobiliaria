package com.propmanager.core.common

import com.propmanager.core.model.ValidationResult
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.types.shouldBeInstanceOf
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.int
import io.kotest.property.arbitrary.localDate
import io.kotest.property.arbitrary.string
import io.kotest.property.checkAll
import java.time.LocalDate
import java.time.format.DateTimeFormatter

/**
 * **Validates: Requirements 5.8**
 *
 * Property 11: Contrato date ordering validation
 *
 * For any pair of dates (fechaInicio, fechaFin), the contrato validator SHALL return
 * Invalid when fechaFin is on or before fechaInicio, and SHALL return Valid when
 * fechaFin is strictly after fechaInicio.
 */
class ContratoDateOrderingPropertyTest :
    FreeSpec({

        val apiFormat = DateTimeFormatter.ofPattern("yyyy-MM-dd")

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

        val dateArb: Arb<LocalDate> =
            Arb.localDate(
                minDate = LocalDate.of(2000, 1, 1),
                maxDate = LocalDate.of(2099, 12, 31),
            )

        "Property 11: Contrato date ordering validation" -
            {

                "fechaFin before fechaInicio returns Invalid for fechaFin" {
                    checkAll(
                        100,
                        dateArb,
                        Arb.int(1..3650),
                        nonBlankArb,
                        nonBlankArb,
                        posDecimalArb,
                    ) { inicio, daysBefore, propId, inquId, monto ->

                        val fechaInicio = inicio
                        val fechaFin = inicio.minusDays(daysBefore.toLong())

                        val result =
                            ContratoValidator.validateCreate(
                                propiedadId = propId,
                                inquilinoId = inquId,
                                fechaInicio = fechaInicio.format(apiFormat),
                                fechaFin = fechaFin.format(apiFormat),
                                montoMensual = monto,
                            )

                        result["fechaFin"].shouldBeInstanceOf<ValidationResult.Invalid>()
                    }
                }

                "fechaFin equal to fechaInicio returns Invalid for fechaFin" {
                    checkAll(100, dateArb, nonBlankArb, nonBlankArb, posDecimalArb) { date, propId, inquId, monto ->

                        val dateStr = date.format(apiFormat)

                        val result =
                            ContratoValidator.validateCreate(
                                propiedadId = propId,
                                inquilinoId = inquId,
                                fechaInicio = dateStr,
                                fechaFin = dateStr,
                                montoMensual = monto,
                            )

                        result["fechaFin"].shouldBeInstanceOf<ValidationResult.Invalid>()
                    }
                }

                "fechaFin after fechaInicio returns Valid for fechaFin" {
                    checkAll(
                        100,
                        dateArb,
                        Arb.int(1..3650),
                        nonBlankArb,
                        nonBlankArb,
                        posDecimalArb,
                    ) { inicio, daysAfter, propId, inquId, monto ->

                        val fechaInicio = inicio
                        val fechaFin = inicio.plusDays(daysAfter.toLong())

                        val result =
                            ContratoValidator.validateCreate(
                                propiedadId = propId,
                                inquilinoId = inquId,
                                fechaInicio = fechaInicio.format(apiFormat),
                                fechaFin = fechaFin.format(apiFormat),
                                montoMensual = monto,
                            )

                        result["fechaFin"].shouldBeInstanceOf<ValidationResult.Valid>()
                    }
                }
            }
    })
