package com.propmanager.core.database.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import com.propmanager.core.database.entity.ContratoEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface ContratoDao {
    @Query("SELECT * FROM contratos WHERE is_deleted = 0 ORDER BY fecha_inicio DESC")
    fun observeAll(): Flow<List<ContratoEntity>>

    @Query("SELECT * FROM contratos WHERE id = :id")
    fun observeById(id: String): Flow<ContratoEntity?>

    @Query(
        """
        SELECT * FROM contratos
        WHERE is_deleted = 0
          AND fecha_fin >= :todayStr
          AND fecha_fin <= :thresholdStr
        ORDER BY fecha_fin ASC
        """,
    )
    fun observeExpiring(
        todayStr: String,
        thresholdStr: String,
    ): Flow<List<ContratoEntity>>

    @Upsert
    suspend fun upsert(entity: ContratoEntity)

    @Upsert
    suspend fun upsertAll(entities: List<ContratoEntity>)

    @Query("UPDATE contratos SET is_deleted = 1 WHERE id = :id")
    suspend fun markDeleted(id: String)

    @Query("DELETE FROM contratos")
    suspend fun deleteAll()
}
