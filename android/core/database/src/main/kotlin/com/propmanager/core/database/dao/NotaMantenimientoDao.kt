package com.propmanager.core.database.dao

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.Query
import com.propmanager.core.database.entity.NotaMantenimientoEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface NotaMantenimientoDao {
    @Query(
        "SELECT * FROM notas_mantenimiento WHERE solicitud_id = :solicitudId ORDER BY created_at ASC"
    )
    fun observeBySolicitudId(solicitudId: String): Flow<List<NotaMantenimientoEntity>>

    @Insert suspend fun insert(entity: NotaMantenimientoEntity)
}
