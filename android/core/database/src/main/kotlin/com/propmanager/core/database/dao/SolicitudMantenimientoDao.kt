package com.propmanager.core.database.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import com.propmanager.core.database.entity.SolicitudMantenimientoEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface SolicitudMantenimientoDao {
    @Query("SELECT * FROM solicitudes_mantenimiento WHERE is_deleted = 0 ORDER BY created_at DESC")
    fun observeAll(): Flow<List<SolicitudMantenimientoEntity>>

    @Query(
        """
        SELECT * FROM solicitudes_mantenimiento
        WHERE is_deleted = 0
          AND (:estado IS NULL OR estado = :estado)
          AND (:prioridad IS NULL OR prioridad = :prioridad)
          AND (:propiedadId IS NULL OR propiedad_id = :propiedadId)
        ORDER BY created_at DESC
        """
    )
    fun observeFiltered(
        estado: String?,
        prioridad: String?,
        propiedadId: String?,
    ): Flow<List<SolicitudMantenimientoEntity>>

    @Query("SELECT * FROM solicitudes_mantenimiento WHERE id = :id")
    fun observeById(id: String): Flow<SolicitudMantenimientoEntity?>

    @Upsert suspend fun upsert(entity: SolicitudMantenimientoEntity)

    @Upsert suspend fun upsertAll(entities: List<SolicitudMantenimientoEntity>)

    @Query("UPDATE solicitudes_mantenimiento SET is_deleted = 1 WHERE id = :id")
    suspend fun markDeleted(id: String)

    @Query("DELETE FROM solicitudes_mantenimiento") suspend fun deleteAll()
}
