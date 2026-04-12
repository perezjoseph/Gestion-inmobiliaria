package com.propmanager.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "gastos",
    foreignKeys = [
        ForeignKey(
            entity = PropiedadEntity::class,
            parentColumns = ["id"],
            childColumns = ["propiedad_id"]
        )
    ],
    indices = [Index("propiedad_id")]
)
data class GastoEntity(
    @PrimaryKey val id: String,
    @ColumnInfo(name = "propiedad_id") val propiedadId: String,
    @ColumnInfo(name = "unidad_id") val unidadId: String?,
    val categoria: String,
    val descripcion: String,
    val monto: String,
    val moneda: String,
    @ColumnInfo(name = "fecha_gasto") val fechaGasto: String,
    val estado: String,
    val proveedor: String?,
    @ColumnInfo(name = "numero_factura") val numeroFactura: String?,
    val notas: String?,
    @ColumnInfo(name = "created_at") val createdAt: Long,
    @ColumnInfo(name = "updated_at") val updatedAt: Long,
    @ColumnInfo(name = "is_deleted") val isDeleted: Boolean = false,
    @ColumnInfo(name = "is_pending_sync") val isPendingSync: Boolean = false
)
