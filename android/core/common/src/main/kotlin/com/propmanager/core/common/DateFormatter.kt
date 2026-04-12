package com.propmanager.core.common

import java.time.LocalDate
import java.time.format.DateTimeFormatter

object DateFormatter {
    private val displayFormat = DateTimeFormatter.ofPattern("dd/MM/yyyy")
    private val apiFormat = DateTimeFormatter.ISO_LOCAL_DATE

    fun toDisplay(date: LocalDate): String = date.format(displayFormat)

    fun toApi(date: LocalDate): String = date.format(apiFormat)

    fun fromApi(dateString: String): LocalDate = LocalDate.parse(dateString, apiFormat)

    fun fromDisplay(dateString: String): LocalDate = LocalDate.parse(dateString, displayFormat)
}
