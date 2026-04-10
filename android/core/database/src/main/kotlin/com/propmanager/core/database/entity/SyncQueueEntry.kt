package com.propmanager.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "sync_queue")
data class SyncQueueEntry(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    @ColumnInfo(name = "entity_type") val entityType: String,
    @ColumnInfo(name = "entity_id") val entityId: String,
    val operation: String,
    val payload: String,
    @ColumnInfo(name = "created_at") val createdAt: Long,
    @ColumnInfo(name = "retry_count") val retryCount: Int = 0,
)
