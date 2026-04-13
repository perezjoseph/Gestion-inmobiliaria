package com.propmanager.feature.scanner

import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.floats.shouldBeGreaterThanOrEqual
import io.kotest.matchers.floats.shouldBeLessThanOrEqual
import io.kotest.matchers.nulls.shouldBeNull
import io.kotest.matchers.nulls.shouldNotBeNull
import io.kotest.matchers.shouldBe
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.element
import io.kotest.property.arbitrary.int
import io.kotest.property.arbitrary.string
import io.kotest.property.checkAll
import java.math.BigDecimal
import java.math.RoundingMode

/**
 * **Validates: Requirements 14.3**
 *
 * Property 15: Receipt OCR text parsing
 *
 * For any text block containing a currency amount pattern (e.g., "RD$ 1,500.00" or "$1500"), a date
 * pattern, and a provider name, ReceiptOcrExtractor.parseReceiptLines() extracts a ReceiptOcrResult
 * where the monto matches the numeric value from the amount pattern and the fecha matches the date
 * in the text.
 */
class ReceiptOcrPropertyTest :
    FreeSpec({
        val extractor = ReceiptOcrExtractor()

        val currencyPrefixArb: Arb<String> = Arb.element("RD$", "US$", "$")

        val receiptAmountArb: Arb<BigDecimal> = arbitrary {
            val intPart = Arb.int(1..999999).bind()
            val decPart = Arb.int(0..99).bind()
            BigDecimal("$intPart.${decPart.toString().padStart(2, '0')}")
        }

        val dayArb: Arb<Int> = Arb.int(1..28)
        val monthArb: Arb<Int> = Arb.int(1..12)
        val yearArb: Arb<Int> = Arb.int(2020..2030)

        val facturaNumberArb: Arb<String> = arbitrary {
            val prefix = Arb.element("A", "B", "NCF", "FAC", "INV").bind()
            val num = Arb.int(1..999999).bind().toString().padStart(6, '0')
            "$prefix-$num"
        }

        "Property 15: Receipt OCR text parsing" -
            {
                "text with currency amount extracts monto correctly" {
                    checkAll(100, currencyPrefixArb, receiptAmountArb) { prefix, amount ->
                        val formattedAmount =
                            amount.setScale(2, RoundingMode.HALF_UP).toPlainString()
                        val lines = listOf("$prefix $formattedAmount")
                        val result = extractor.parseReceiptLines(lines)
                        result.monto.shouldNotBeNull()
                        result.monto!!.compareTo(amount) shouldBe 0
                    }
                }

                "text with DD/MM/YYYY date extracts fecha correctly" {
                    checkAll(100, dayArb, monthArb, yearArb) { day, month, year ->
                        val dateStr =
                            "${day.toString().padStart(2, '0')}/${month.toString().padStart(2, '0')}/$year"
                        val lines = listOf("Fecha: $dateStr")
                        val result = extractor.parseReceiptLines(lines)
                        result.fecha.shouldNotBeNull()
                        result.fecha!!.dayOfMonth shouldBe day
                        result.fecha!!.monthValue shouldBe month
                        result.fecha!!.year shouldBe year
                    }
                }

                "text with Factura or NCF pattern extracts numeroFactura" {
                    checkAll(100, facturaNumberArb) { facturaNum ->
                        val lines = listOf("Factura: $facturaNum")
                        val result = extractor.parseReceiptLines(lines)
                        result.numeroFactura.shouldNotBeNull()
                        result.numeroFactura shouldBe facturaNum
                    }
                }

                "random garbage text does not crash and returns a result" {
                    checkAll(100, Arb.string(0..100)) { garbage ->
                        val lines = listOf(garbage)
                        val result = extractor.parseReceiptLines(lines)
                        result.confidence shouldBeGreaterThanOrEqual 0.0f
                        result.confidence shouldBeLessThanOrEqual 1.0f
                    }
                }

                "empty input returns all null fields with zero confidence" {
                    val result = extractor.parseReceiptLines(emptyList())
                    result.monto.shouldBeNull()
                    result.fecha.shouldBeNull()
                    result.proveedor.shouldBeNull()
                    result.numeroFactura.shouldBeNull()
                    result.confidence shouldBe 0.0f
                }
            }
    })
