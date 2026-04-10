package com.propmanager.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "dashboard_cache")
data class DashboardCache(
    @PrimaryKey val key: String,
    val data: String,
    @ColumnInfo(name = "cached_at") val cachedAt: Long,
)
