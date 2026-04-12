package com.propmanager.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "notas_mantenimiento",
    foreignKeys = [
        ForeignKey(
            entity = SolicitudMantenimientoEntity::class,
            parentColumns = ["id"],
            childColumns = ["solicitud_id"]
        )
    ],
    indices = [Index("solicitud_id")]
)
data class NotaMantenimientoEntity(
    @PrimaryKey val id: String,
    @ColumnInfo(name = "solicitud_id") val solicitudId: String,
    @ColumnInfo(name = "autor_id") val autorId: String,
    val contenido: String,
    @ColumnInfo(name = "created_at") val createdAt: Long
)
