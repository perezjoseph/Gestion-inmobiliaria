package com.propmanager.core.ui.components

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import com.propmanager.core.common.CurrencyFormatter
import java.math.BigDecimal

@Composable
fun CurrencyText(
    amount: BigDecimal,
    currency: String,
    modifier: Modifier = Modifier,
    style: TextStyle = MaterialTheme.typography.bodyLarge,
) {
    val formatted = remember(amount, currency) {
        CurrencyFormatter.format(amount, currency)
    }
    Text(
        text = formatted,
        style = style,
        modifier = modifier,
    )
}
