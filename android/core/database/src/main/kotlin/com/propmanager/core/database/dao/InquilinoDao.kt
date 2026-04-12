package com.propmanager.core.database.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import com.propmanager.core.database.entity.InquilinoEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface InquilinoDao {

    @Query("SELECT * FROM inquilinos WHERE is_deleted = 0 ORDER BY apellido ASC, nombre ASC")
    fun observeAll(): Flow<List<InquilinoEntity>>

    @Query("SELECT * FROM inquilinos WHERE id = :id")
    fun observeById(id: String): Flow<InquilinoEntity?>

    @Query(
        """
        SELECT * FROM inquilinos
        WHERE is_deleted = 0
          AND (nombre LIKE '%' || :query || '%'
               OR apellido LIKE '%' || :query || '%'
               OR cedula LIKE '%' || :query || '%')
        ORDER BY apellido ASC, nombre ASC
        """
    )
    fun search(query: String): Flow<List<InquilinoEntity>>

    @Upsert
    suspend fun upsert(entity: InquilinoEntity)

    @Upsert
    suspend fun upsertAll(entities: List<InquilinoEntity>)

    @Query("UPDATE inquilinos SET is_deleted = 1 WHERE id = :id")
    suspend fun markDeleted(id: String)

    @Query("DELETE FROM inquilinos")
    suspend fun deleteAll()
}
