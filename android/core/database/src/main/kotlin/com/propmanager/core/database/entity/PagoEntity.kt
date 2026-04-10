package com.propmanager.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "pagos",
    foreignKeys =
        [
            ForeignKey(
                entity = ContratoEntity::class,
                parentColumns = ["id"],
                childColumns = ["contrato_id"],
            )
        ],
    indices = [Index("contrato_id")],
)
data class PagoEntity(
    @PrimaryKey val id: String,
    @ColumnInfo(name = "contrato_id") val contratoId: String,
    val monto: String,
    val moneda: String,
    @ColumnInfo(name = "fecha_pago") val fechaPago: String?,
    @ColumnInfo(name = "fecha_vencimiento") val fechaVencimiento: String,
    @ColumnInfo(name = "metodo_pago") val metodoPago: String?,
    val estado: String,
    val notas: String?,
    @ColumnInfo(name = "created_at") val createdAt: Long,
    @ColumnInfo(name = "updated_at") val updatedAt: Long,
    @ColumnInfo(name = "is_deleted") val isDeleted: Boolean = false,
    @ColumnInfo(name = "is_pending_sync") val isPendingSync: Boolean = false,
)
