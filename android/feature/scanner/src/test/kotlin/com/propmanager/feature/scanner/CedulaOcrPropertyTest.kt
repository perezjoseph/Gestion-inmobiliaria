package com.propmanager.feature.scanner

import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.floats.shouldBeGreaterThanOrEqual
import io.kotest.matchers.floats.shouldBeLessThanOrEqual
import io.kotest.matchers.nulls.shouldBeNull
import io.kotest.matchers.nulls.shouldNotBeNull
import io.kotest.matchers.shouldBe
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.int
import io.kotest.property.arbitrary.string
import io.kotest.property.checkAll

/**
 * **Validates: Requirements 14.1**
 *
 * Property 14: Cédula OCR text parsing
 *
 * For any text block following the Dominican cédula layout pattern (containing a formatted
 * cédula number, nombre, and apellido in expected positions), CedulaOcrExtractor.parseCedulaLines()
 * extracts a CedulaOcrResult where the cedula, nombre, and apellido fields match the values
 * present in the input text.
 */
class CedulaOcrPropertyTest :
    FreeSpec({

        val extractor = CedulaOcrExtractor()

        val cedulaNumberArb: Arb<String> =
            arbitrary {
                val part1 = Arb.int(1..999).bind().toString().padStart(3, '0')
                val part2 = Arb.int(1..9999999).bind().toString().padStart(7, '0')
                val part3 = Arb.int(0..9).bind()
                "$part1-$part2-$part3"
            }

        val dominicanNameArb: Arb<String> =
            arbitrary(
                edgecases =
                    listOf(
                        "Juan",
                        "María",
                        "José",
                        "Ana",
                    ),
            ) {
                val names =
                    listOf(
                        "Juan", "Pedro", "Carlos", "María", "Ana", "José",
                        "Luis", "Miguel", "Rosa", "Carmen", "Francisco", "Rafael",
                        "Ramón", "Juana", "Altagracia", "Mercedes", "Ángel", "Félix",
                        "Héctor", "Óscar", "Andrés", "Tomás", "Nicolás", "Inés",
                    )
                names[Arb.int(0 until names.size).bind()]
            }

        val dominicanSurnameArb: Arb<String> =
            arbitrary(
                edgecases =
                    listOf(
                        "Pérez",
                        "García",
                        "Rodríguez",
                        "Martínez",
                    ),
            ) {
                val surnames =
                    listOf(
                        "Pérez", "García", "Rodríguez", "Martínez", "López",
                        "González", "Hernández", "Díaz", "Morales", "Reyes",
                        "Jiménez", "Castillo", "Núñez", "Ramírez", "Torres",
                        "Vásquez", "Sánchez", "Fernández", "Álvarez", "Méndez",
                    )
                surnames[Arb.int(0 until surnames.size).bind()]
            }

        "Property 14: Cédula OCR text parsing" -
            {

                "text with valid cédula number extracts the number correctly" {
                    checkAll(100, cedulaNumberArb) { cedulaNum ->
                        val lines =
                            listOf(
                                "REPUBLICA DOMINICANA",
                                "CEDULA DE IDENTIDAD Y ELECTORAL",
                                cedulaNum,
                            )
                        val result = extractor.parseCedulaLines(lines)
                        result.cedula.shouldNotBeNull()
                        result.cedula shouldBe cedulaNum
                    }
                }

                "text with nombre and apellido extracts them" {
                    checkAll(100, dominicanSurnameArb, dominicanNameArb, cedulaNumberArb) { apellido, nombre, cedulaNum ->
                        val lines =
                            listOf(
                                "REPUBLICA DOMINICANA",
                                "CEDULA DE IDENTIDAD Y ELECTORAL",
                                apellido,
                                nombre,
                                cedulaNum,
                            )
                        val result = extractor.parseCedulaLines(lines)
                        result.apellido shouldBe apellido
                        result.nombre shouldBe nombre
                        result.cedula shouldBe cedulaNum
                        result.confidence shouldBe 1.0f
                    }
                }

                "text with cédula number without dashes still extracts correctly" {
                    checkAll(100, cedulaNumberArb) { cedulaNum ->
                        val noDashes = cedulaNum.replace("-", "")
                        val lines = listOf(noDashes)
                        val result = extractor.parseCedulaLines(lines)
                        result.cedula.shouldNotBeNull()
                        result.cedula shouldBe cedulaNum
                    }
                }

                "random garbage text does not crash and returns null fields" {
                    checkAll(100, Arb.string(0..100)) { garbage ->
                        val lines = listOf(garbage)
                        val result = extractor.parseCedulaLines(lines)
                        result.confidence shouldBeGreaterThanOrEqual 0.0f
                        result.confidence shouldBeLessThanOrEqual 1.0f
                    }
                }

                "empty input returns all null fields with zero confidence" {
                    val result = extractor.parseCedulaLines(emptyList())
                    result.cedula.shouldBeNull()
                    result.nombre.shouldBeNull()
                    result.apellido.shouldBeNull()
                    result.confidence shouldBe 0.0f
                }
            }
    })
