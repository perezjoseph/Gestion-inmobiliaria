package com.propmanager.core.database.entity

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "solicitudes_mantenimiento",
    foreignKeys =
        [
            ForeignKey(
                entity = PropiedadEntity::class,
                parentColumns = ["id"],
                childColumns = ["propiedad_id"],
            )
        ],
    indices = [Index("propiedad_id")],
)
data class SolicitudMantenimientoEntity(
    @PrimaryKey val id: String,
    @ColumnInfo(name = "propiedad_id") val propiedadId: String,
    @ColumnInfo(name = "unidad_id") val unidadId: String?,
    @ColumnInfo(name = "inquilino_id") val inquilinoId: String?,
    val titulo: String,
    val descripcion: String?,
    val estado: String,
    val prioridad: String,
    @ColumnInfo(name = "nombre_proveedor") val nombreProveedor: String?,
    @ColumnInfo(name = "telefono_proveedor") val telefonoProveedor: String?,
    @ColumnInfo(name = "email_proveedor") val emailProveedor: String?,
    @ColumnInfo(name = "costo_monto") val costoMonto: String?,
    @ColumnInfo(name = "costo_moneda") val costoMoneda: String?,
    @ColumnInfo(name = "fecha_inicio") val fechaInicio: Long?,
    @ColumnInfo(name = "fecha_fin") val fechaFin: Long?,
    @ColumnInfo(name = "created_at") val createdAt: Long,
    @ColumnInfo(name = "updated_at") val updatedAt: Long,
    @ColumnInfo(name = "is_deleted") val isDeleted: Boolean = false,
    @ColumnInfo(name = "is_pending_sync") val isPendingSync: Boolean = false,
)
