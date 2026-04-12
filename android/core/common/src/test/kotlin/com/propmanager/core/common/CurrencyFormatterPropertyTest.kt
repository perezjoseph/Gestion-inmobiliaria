package com.propmanager.core.common

import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.string.shouldContain
import io.kotest.matchers.string.shouldStartWith
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.element
import io.kotest.property.arbitrary.long
import io.kotest.property.checkAll
import java.math.BigDecimal
import java.math.RoundingMode

/**
 * **Validates: Requirements 15.3**
 *
 * Property 13: Currency formatting correctness
 *
 * For any non-negative BigDecimal amount and currency code in {"DOP", "USD"},
 * CurrencyFormatter.format(amount, currency) produces a string that starts with
 * "RD$" for DOP or "US$" for USD, contains the amount with exactly 2 decimal places,
 * and uses proper thousands separators.
 */
class CurrencyFormatterPropertyTest :
    FreeSpec({

        val currencyArb: Arb<String> = Arb.element("DOP", "USD")

        val nonNegativeBigDecimalArb: Arb<BigDecimal> =
            arbitrary(
                edgecases =
                    listOf(
                        BigDecimal.ZERO,
                        BigDecimal("0.01"),
                        BigDecimal("999.99"),
                        BigDecimal("1000.00"),
                        BigDecimal("1234567.89"),
                    ),
            ) {
                val cents = Arb.long(0L..99999999999L).bind()
                BigDecimal(cents).divide(BigDecimal(100))
            }

        "Property 13: Currency formatting correctness" -
            {

                "formatted output starts with correct currency symbol" {
                    checkAll(100, nonNegativeBigDecimalArb, currencyArb) { amount, currency ->
                        val result = CurrencyFormatter.format(amount, currency)
                        val expectedSymbol = if (currency == "DOP") "RD$" else "US$"
                        result shouldStartWith "$expectedSymbol "
                    }
                }

                "formatted output contains exactly 2 decimal places" {
                    checkAll(100, nonNegativeBigDecimalArb, currencyArb) { amount, currency ->
                        val result = CurrencyFormatter.format(amount, currency)
                        val decimalPart = result.substringAfterLast(".")
                        decimalPart.length shouldBe 2
                    }
                }

                "formatted output uses thousands separators for amounts >= 1000" {
                    val largeAmountArb: Arb<BigDecimal> =
                        arbitrary {
                            val cents = Arb.long(100000L..99999999999L).bind()
                            BigDecimal(cents).divide(BigDecimal(100))
                        }

                    checkAll(100, largeAmountArb, currencyArb) { amount, currency ->
                        val result = CurrencyFormatter.format(amount, currency)
                        val numericPart = result.substringAfter(" ")
                        val integerPart = numericPart.substringBefore(".")
                        integerPart shouldContain ","
                    }
                }

                "formatted decimal value matches input amount rounded to 2 places" {
                    checkAll(100, nonNegativeBigDecimalArb, currencyArb) { amount, currency ->
                        val result = CurrencyFormatter.format(amount, currency)
                        val numericPart = result.substringAfter(" ").replace(",", "")
                        val parsedBack = BigDecimal(numericPart)
                        val expected = amount.setScale(2, RoundingMode.HALF_EVEN)
                        parsedBack.compareTo(expected) shouldBe 0
                    }
                }
            }
    })
