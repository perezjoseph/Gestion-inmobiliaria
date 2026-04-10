package com.propmanager.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "contratos",
    foreignKeys =
        [
            ForeignKey(
                entity = PropiedadEntity::class,
                parentColumns = ["id"],
                childColumns = ["propiedad_id"],
            ),
            ForeignKey(
                entity = InquilinoEntity::class,
                parentColumns = ["id"],
                childColumns = ["inquilino_id"],
            ),
        ],
    indices = [Index("propiedad_id"), Index("inquilino_id")],
)
data class ContratoEntity(
    @PrimaryKey val id: String,
    @ColumnInfo(name = "propiedad_id") val propiedadId: String,
    @ColumnInfo(name = "inquilino_id") val inquilinoId: String,
    @ColumnInfo(name = "fecha_inicio") val fechaInicio: String,
    @ColumnInfo(name = "fecha_fin") val fechaFin: String,
    @ColumnInfo(name = "monto_mensual") val montoMensual: String,
    val deposito: String?,
    val moneda: String,
    val estado: String,
    @ColumnInfo(name = "created_at") val createdAt: Long,
    @ColumnInfo(name = "updated_at") val updatedAt: Long,
    @ColumnInfo(name = "is_deleted") val isDeleted: Boolean = false,
    @ColumnInfo(name = "is_pending_sync") val isPendingSync: Boolean = false,
)
