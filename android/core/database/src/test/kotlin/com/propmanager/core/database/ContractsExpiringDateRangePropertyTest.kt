package com.propmanager.core.database

import com.propmanager.core.database.entity.ContratoEntity
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.collections.shouldContainExactlyInAnyOrder
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.element
import io.kotest.property.arbitrary.int
import io.kotest.property.arbitrary.list
import io.kotest.property.arbitrary.long
import io.kotest.property.arbitrary.uuid
import io.kotest.property.checkAll
import java.time.LocalDate
import java.time.format.DateTimeFormatter

/**
 * **Validates: Requirements 5.6**
 *
 * Property 9: Contracts-expiring date range filter
 *
 * For any set of contratos and any positive integer day threshold, querying for expiring contracts
 * returns exactly those contratos whose fecha_fin falls between today (inclusive) and today +
 * threshold days (inclusive), and excludes contratos with fecha_fin outside that range.
 */
class ContractsExpiringDateRangePropertyTest :
    FreeSpec({
        val apiFormat: DateTimeFormatter = DateTimeFormatter.ISO_LOCAL_DATE
        val estados = listOf("activo", "terminado", "renovado", "vencido")

        fun dateToString(date: LocalDate): String = date.format(apiFormat)

        fun filterExpiring(
            contratos: List<ContratoEntity>,
            referenceDate: LocalDate,
            thresholdDays: Int,
        ): List<ContratoEntity> {
            val rangeStart = referenceDate
            val rangeEnd = referenceDate.plusDays(thresholdDays.toLong())
            return contratos.filter { c ->
                val fechaFin = LocalDate.parse(c.fechaFin, apiFormat)
                !fechaFin.isBefore(rangeStart) && !fechaFin.isAfter(rangeEnd)
            }
        }

        // Generate a contrato with fecha_fin offset from a reference date by a random number of
        // days
        fun contratoArb(referenceDate: LocalDate): Arb<ContratoEntity> = arbitrary {
            // Offset range: -60 to +120 days from reference, giving good spread around the window
            val offsetDays = Arb.int(-60..120).bind()
            val fechaFin = referenceDate.plusDays(offsetDays.toLong())
            val fechaInicio = fechaFin.minusMonths(12)
            ContratoEntity(
                id = Arb.uuid().bind().toString(),
                propiedadId = Arb.uuid().bind().toString(),
                inquilinoId = Arb.uuid().bind().toString(),
                fechaInicio = dateToString(fechaInicio),
                fechaFin = dateToString(fechaFin),
                montoMensual = "25000.00",
                deposito = "50000.00",
                moneda = Arb.element("DOP", "USD").bind(),
                estado = Arb.element(estados).bind(),
                createdAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
                updatedAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
                isDeleted = false,
                isPendingSync = false,
            )
        }

        "Property 9: Contracts-expiring date range filter" -
            {
                "returns exactly contratos with fecha_fin in [today, today + threshold]" {
                    val today = LocalDate.now()
                    checkAll(100, Arb.list(contratoArb(today), 1..30), Arb.int(1..90)) {
                        contratos,
                        threshold ->
                        val result = filterExpiring(contratos, today, threshold)
                        val rangeStart = today
                        val rangeEnd = today.plusDays(threshold.toLong())

                        result.forEach { c ->
                            val fechaFin = LocalDate.parse(c.fechaFin, apiFormat)
                            assert(!fechaFin.isBefore(rangeStart)) {
                                "fecha_fin $fechaFin should not be before $rangeStart"
                            }
                            assert(!fechaFin.isAfter(rangeEnd)) {
                                "fecha_fin $fechaFin should not be after $rangeEnd"
                            }
                        }

                        val expected =
                            contratos.filter { c ->
                                val fechaFin = LocalDate.parse(c.fechaFin, apiFormat)
                                !fechaFin.isBefore(rangeStart) && !fechaFin.isAfter(rangeEnd)
                            }
                        result shouldContainExactlyInAnyOrder expected
                    }
                }

                "excludes contratos with fecha_fin before today" {
                    val today = LocalDate.now()
                    checkAll(100, Arb.int(1..90)) { threshold ->
                        val pastContrato =
                            ContratoEntity(
                                id = "past-1",
                                propiedadId = "prop-1",
                                inquilinoId = "inq-1",
                                fechaInicio = dateToString(today.minusYears(2)),
                                fechaFin = dateToString(today.minusDays(1)),
                                montoMensual = "20000.00",
                                deposito = null,
                                moneda = "DOP",
                                estado = "activo",
                                createdAt = 1000000L,
                                updatedAt = 1000000L,
                            )
                        val result = filterExpiring(listOf(pastContrato), today, threshold)
                        assert(result.isEmpty()) {
                            "Contract with fecha_fin before today should be excluded"
                        }
                    }
                }

                "excludes contratos with fecha_fin after today + threshold" {
                    val today = LocalDate.now()
                    checkAll(100, Arb.int(1..90)) { threshold ->
                        val futureContrato =
                            ContratoEntity(
                                id = "future-1",
                                propiedadId = "prop-1",
                                inquilinoId = "inq-1",
                                fechaInicio = dateToString(today),
                                fechaFin = dateToString(today.plusDays(threshold.toLong() + 1)),
                                montoMensual = "20000.00",
                                deposito = null,
                                moneda = "DOP",
                                estado = "activo",
                                createdAt = 1000000L,
                                updatedAt = 1000000L,
                            )
                        val result = filterExpiring(listOf(futureContrato), today, threshold)
                        assert(result.isEmpty()) {
                            "Contract with fecha_fin after today+threshold should be excluded"
                        }
                    }
                }

                "includes contratos on boundary dates (today and today + threshold)" {
                    val today = LocalDate.now()
                    checkAll(100, Arb.int(1..90)) { threshold ->
                        val boundaryToday =
                            ContratoEntity(
                                id = "boundary-today",
                                propiedadId = "prop-1",
                                inquilinoId = "inq-1",
                                fechaInicio = dateToString(today.minusYears(1)),
                                fechaFin = dateToString(today),
                                montoMensual = "20000.00",
                                deposito = null,
                                moneda = "DOP",
                                estado = "activo",
                                createdAt = 1000000L,
                                updatedAt = 1000000L,
                            )
                        val boundaryEnd =
                            ContratoEntity(
                                id = "boundary-end",
                                propiedadId = "prop-2",
                                inquilinoId = "inq-2",
                                fechaInicio = dateToString(today),
                                fechaFin = dateToString(today.plusDays(threshold.toLong())),
                                montoMensual = "30000.00",
                                deposito = null,
                                moneda = "USD",
                                estado = "activo",
                                createdAt = 1000000L,
                                updatedAt = 1000000L,
                            )
                        val result =
                            filterExpiring(listOf(boundaryToday, boundaryEnd), today, threshold)
                        assert(result.size == 2) {
                            "Both boundary contracts should be included, got ${result.size}"
                        }
                    }
                }

                "default 30-day threshold works correctly with mixed contratos" {
                    val today = LocalDate.now()
                    val defaultThreshold = 30
                    checkAll(100, Arb.list(contratoArb(today), 5..30)) { contratos ->
                        val result = filterExpiring(contratos, today, defaultThreshold)
                        val rangeEnd = today.plusDays(30)

                        result.forEach { c ->
                            val fechaFin = LocalDate.parse(c.fechaFin, apiFormat)
                            assert(!fechaFin.isBefore(today) && !fechaFin.isAfter(rangeEnd))
                        }

                        val missed =
                            contratos.filter { c ->
                                val fechaFin = LocalDate.parse(c.fechaFin, apiFormat)
                                !fechaFin.isBefore(today) && !fechaFin.isAfter(rangeEnd)
                            } - result.toSet()
                        assert(missed.isEmpty()) { "Filter should not miss any matching contratos" }
                    }
                }
            }
    })
