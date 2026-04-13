package com.propmanager.core.common

import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.string.shouldMatch
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.int
import io.kotest.property.checkAll
import java.time.LocalDate

/**
 * **Validates: Requirements 15.2, 15.4, 15.5**
 *
 * Property 12: Date formatting round-trip
 *
 * For any valid LocalDate, formatting to display format (DD/MM/YYYY) then parsing back yields the
 * original date. Formatting to API format (YYYY-MM-DD) then parsing back yields the original date.
 * Display format matches \d{2}/\d{2}/\d{4} and API format matches \d{4}-\d{2}-\d{2}.
 */
class DateFormatterPropertyTest :
    FreeSpec({
        val localDateArb: Arb<LocalDate> =
            arbitrary(
                edgecases =
                    listOf(
                        LocalDate.of(2000, 1, 1),
                        LocalDate.of(2024, 2, 29),
                        LocalDate.of(1970, 1, 1),
                        LocalDate.of(2099, 12, 31),
                    )
            ) {
                val year = Arb.int(1900..2099).bind()
                val month = Arb.int(1..12).bind()
                val maxDay = LocalDate.of(year, month, 1).lengthOfMonth()
                val day = Arb.int(1..maxDay).bind()
                LocalDate.of(year, month, day)
            }

        "Property 12: Date formatting round-trip" -
            {
                "display format round-trip preserves date" {
                    checkAll(100, localDateArb) { date ->
                        val formatted = DateFormatter.toDisplay(date)
                        val parsed = DateFormatter.fromDisplay(formatted)
                        parsed shouldBe date
                    }
                }

                "API format round-trip preserves date" {
                    checkAll(100, localDateArb) { date ->
                        val formatted = DateFormatter.toApi(date)
                        val parsed = DateFormatter.fromApi(formatted)
                        parsed shouldBe date
                    }
                }

                "display format matches DD/MM/YYYY regex" {
                    checkAll(100, localDateArb) { date ->
                        val formatted = DateFormatter.toDisplay(date)
                        formatted shouldMatch Regex("""\d{2}/\d{2}/\d{4}""")
                    }
                }

                "API format matches YYYY-MM-DD regex" {
                    checkAll(100, localDateArb) { date ->
                        val formatted = DateFormatter.toApi(date)
                        formatted shouldMatch Regex("""\d{4}-\d{2}-\d{2}""")
                    }
                }
            }
    })
