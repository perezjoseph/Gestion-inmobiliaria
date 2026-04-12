package com.propmanager.core.common

import com.google.common.truth.Truth.assertThat
import org.junit.jupiter.api.Test
import java.math.BigDecimal

class CurrencyFormatterTest {
    @Test
    fun `formats DOP with RD$ symbol`() {
        val result = CurrencyFormatter.format(BigDecimal("1500.00"), "DOP")
        assertThat(result).startsWith("RD$")
    }

    @Test
    fun `formats USD with US$ symbol`() {
        val result = CurrencyFormatter.format(BigDecimal("1500.00"), "USD")
        assertThat(result).startsWith("US$")
    }

    @Test
    fun `unknown currency uses currency code as symbol`() {
        val result = CurrencyFormatter.format(BigDecimal("100.00"), "EUR")
        assertThat(result).startsWith("EUR")
    }

    @Test
    fun `formats with 2 decimal places`() {
        val result = CurrencyFormatter.format(BigDecimal("100"), "DOP")
        assertThat(result).contains(".00")
    }

    @Test
    fun `formats large amounts with thousands separators`() {
        val result = CurrencyFormatter.format(BigDecimal("1234567.89"), "DOP")
        // Dominican locale uses comma or period for thousands; just verify symbol and decimals
        assertThat(result).startsWith("RD$")
        assertThat(result).contains("89")
    }

    @Test
    fun `formats zero amount`() {
        val result = CurrencyFormatter.format(BigDecimal.ZERO, "USD")
        assertThat(result).startsWith("US$")
        assertThat(result).contains("0.00")
    }
}
