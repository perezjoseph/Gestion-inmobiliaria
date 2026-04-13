package com.propmanager.core.ui.components

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import com.propmanager.core.common.DateFormatter
import com.propmanager.core.ui.R
import java.time.LocalDate

@Suppress("UnusedParameter")
@Composable
fun DatePickerField(
    value: LocalDate?,
    onValueChange: (LocalDate) -> Unit,
    label: String,
    modifier: Modifier = Modifier,
    error: String? = null,
    enabled: Boolean = true,
    onPickerRequest: () -> Unit = {},
) {
    val displayText = value?.let { DateFormatter.toDisplay(it) } ?: ""

    PropManagerTextField(
        value = displayText,
        onValueChange = { /* read-only, changes come from picker */ },
        label = label,
        modifier = modifier,
        error = error,
        enabled = enabled,
        readOnly = true,
        trailingIcon = {
            IconButton(onClick = onPickerRequest, enabled = enabled) {
                Icon(
                    imageVector = Icons.Filled.DateRange,
                    contentDescription = stringResource(R.string.select_date),
                )
            }
        },
    )
}
