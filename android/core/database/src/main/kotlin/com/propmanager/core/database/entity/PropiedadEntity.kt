package com.propmanager.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "propiedades")
data class PropiedadEntity(
    @PrimaryKey val id: String,
    val titulo: String,
    val descripcion: String?,
    val direccion: String,
    val ciudad: String,
    val provincia: String,
    @ColumnInfo(name = "tipo_propiedad") val tipoPropiedad: String,
    val habitaciones: Int?,
    val banos: Int?,
    @ColumnInfo(name = "area_m2") val areaM2: String?,
    val precio: String,
    val moneda: String,
    val estado: String,
    val imagenes: String?,
    @ColumnInfo(name = "created_at") val createdAt: Long,
    @ColumnInfo(name = "updated_at") val updatedAt: Long,
    @ColumnInfo(name = "is_deleted") val isDeleted: Boolean = false,
    @ColumnInfo(name = "is_pending_sync") val isPendingSync: Boolean = false
)
