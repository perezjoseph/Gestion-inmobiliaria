package com.propmanager.core.database.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import com.propmanager.core.database.entity.GastoEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface GastoDao {
    @Query("SELECT * FROM gastos WHERE is_deleted = 0 ORDER BY fecha_gasto DESC")
    fun observeAll(): Flow<List<GastoEntity>>

    @Query(
        """
        SELECT * FROM gastos
        WHERE is_deleted = 0
          AND (:propiedadId IS NULL OR propiedad_id = :propiedadId)
          AND (:categoria IS NULL OR categoria = :categoria)
          AND (:estado IS NULL OR estado = :estado)
          AND (:fechaDesde IS NULL OR fecha_gasto >= :fechaDesde)
          AND (:fechaHasta IS NULL OR fecha_gasto <= :fechaHasta)
        ORDER BY fecha_gasto DESC
        """,
    )
    fun observeFiltered(
        propiedadId: String?,
        categoria: String?,
        estado: String?,
        fechaDesde: String?,
        fechaHasta: String?,
    ): Flow<List<GastoEntity>>

    @Upsert
    suspend fun upsert(entity: GastoEntity)

    @Upsert
    suspend fun upsertAll(entities: List<GastoEntity>)

    @Query("UPDATE gastos SET is_deleted = 1 WHERE id = :id")
    suspend fun markDeleted(id: String)

    @Query("DELETE FROM gastos")
    suspend fun deleteAll()
}
