package com.propmanager.core.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.propmanager.core.ui.theme.SyncPendingColor

@Composable
fun SyncStatusBadge(isPendingSync: Boolean, modifier: Modifier = Modifier) {
    if (!isPendingSync) return

    val description = "Pendiente de sincronización"
    Box(
        modifier =
            modifier.size(8.dp).clip(CircleShape).background(SyncPendingColor).semantics {
                contentDescription = description
            }
    )
}
