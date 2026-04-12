package com.propmanager.core.common

import java.math.BigDecimal
import java.text.NumberFormat
import java.util.Locale

object CurrencyFormatter {
    private val dominicanLocale = Locale("es", "DO")

    fun format(
        amount: BigDecimal,
        currency: String,
    ): String {
        val symbol =
            when (currency) {
                "DOP" -> "RD$"
                "USD" -> "US$"
                else -> currency
            }
        val formatted =
            NumberFormat
                .getNumberInstance(dominicanLocale)
                .apply {
                    minimumFractionDigits = 2
                    maximumFractionDigits = 2
                }.format(amount)
        return "$symbol $formatted"
    }
}
