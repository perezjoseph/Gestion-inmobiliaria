package com.propmanager.core.common

import com.google.common.truth.Truth.assertThat
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import java.time.LocalDate
import java.time.format.DateTimeParseException

class DateFormatterTest {
    @Test
    fun `toDisplay formats date as DD-MM-YYYY`() {
        val date = LocalDate.of(2025, 1, 5)
        assertThat(DateFormatter.toDisplay(date)).isEqualTo("05/01/2025")
    }

    @Test
    fun `toApi formats date as YYYY-MM-DD`() {
        val date = LocalDate.of(2025, 12, 31)
        assertThat(DateFormatter.toApi(date)).isEqualTo("2025-12-31")
    }

    @Test
    fun `fromApi parses YYYY-MM-DD string`() {
        val result = DateFormatter.fromApi("2025-06-15")
        assertThat(result).isEqualTo(LocalDate.of(2025, 6, 15))
    }

    @Test
    fun `fromDisplay parses DD-MM-YYYY string`() {
        val result = DateFormatter.fromDisplay("15/06/2025")
        assertThat(result).isEqualTo(LocalDate.of(2025, 6, 15))
    }

    @Test
    fun `fromApi rejects invalid format`() {
        assertThrows<DateTimeParseException> {
            DateFormatter.fromApi("15/06/2025")
        }
    }

    @Test
    fun `fromDisplay rejects API format`() {
        assertThrows<DateTimeParseException> {
            DateFormatter.fromDisplay("2025-06-15")
        }
    }

    @Test
    fun `round-trip through display format preserves date`() {
        val original = LocalDate.of(2024, 2, 29)
        val roundTripped = DateFormatter.fromDisplay(DateFormatter.toDisplay(original))
        assertThat(roundTripped).isEqualTo(original)
    }

    @Test
    fun `round-trip through API format preserves date`() {
        val original = LocalDate.of(2024, 2, 29)
        val roundTripped = DateFormatter.fromApi(DateFormatter.toApi(original))
        assertThat(roundTripped).isEqualTo(original)
    }
}
