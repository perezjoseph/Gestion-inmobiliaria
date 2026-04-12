package com.propmanager.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "inquilinos")
data class InquilinoEntity(
    @PrimaryKey val id: String,
    val nombre: String,
    val apellido: String,
    val email: String?,
    val telefono: String?,
    val cedula: String,
    @ColumnInfo(name = "contacto_emergencia") val contactoEmergencia: String?,
    val notas: String?,
    @ColumnInfo(name = "created_at") val createdAt: Long,
    @ColumnInfo(name = "updated_at") val updatedAt: Long,
    @ColumnInfo(name = "is_deleted") val isDeleted: Boolean = false,
    @ColumnInfo(name = "is_pending_sync") val isPendingSync: Boolean = false
)
