package com.propmanager.core.database.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import com.propmanager.core.database.entity.PagoEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface PagoDao {

    @Query("SELECT * FROM pagos WHERE is_deleted = 0 ORDER BY fecha_vencimiento DESC")
    fun observeAll(): Flow<List<PagoEntity>>

    @Query(
        """
        SELECT * FROM pagos
        WHERE is_deleted = 0
          AND (:contratoId IS NULL OR contrato_id = :contratoId)
          AND (:estado IS NULL OR estado = :estado)
          AND (:fechaDesde IS NULL OR fecha_vencimiento >= :fechaDesde)
          AND (:fechaHasta IS NULL OR fecha_vencimiento <= :fechaHasta)
        ORDER BY fecha_vencimiento DESC
        """
    )
    fun observeFiltered(
        contratoId: String?,
        estado: String?,
        fechaDesde: String?,
        fechaHasta: String?
    ): Flow<List<PagoEntity>>

    @Upsert
    suspend fun upsert(entity: PagoEntity)

    @Upsert
    suspend fun upsertAll(entities: List<PagoEntity>)

    @Query("UPDATE pagos SET is_deleted = 1 WHERE id = :id")
    suspend fun markDeleted(id: String)

    @Query("DELETE FROM pagos")
    suspend fun deleteAll()
}
