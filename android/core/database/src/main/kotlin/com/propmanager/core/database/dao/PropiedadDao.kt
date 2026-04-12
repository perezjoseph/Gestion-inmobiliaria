package com.propmanager.core.database.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import com.propmanager.core.database.entity.PropiedadEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface PropiedadDao {
    @Query("SELECT * FROM propiedades WHERE is_deleted = 0 ORDER BY titulo ASC")
    fun observeAll(): Flow<List<PropiedadEntity>>

    @Query("SELECT * FROM propiedades WHERE id = :id")
    fun observeById(id: String): Flow<PropiedadEntity?>

    @Query(
        """
        SELECT * FROM propiedades
        WHERE is_deleted = 0
          AND (:ciudad IS NULL OR ciudad = :ciudad)
          AND (:estado IS NULL OR estado = :estado)
          AND (:tipoPropiedad IS NULL OR tipo_propiedad = :tipoPropiedad)
        ORDER BY titulo ASC
        """,
    )
    fun observeFiltered(
        ciudad: String?,
        estado: String?,
        tipoPropiedad: String?,
    ): Flow<List<PropiedadEntity>>

    @Upsert
    suspend fun upsert(entity: PropiedadEntity)

    @Upsert
    suspend fun upsertAll(entities: List<PropiedadEntity>)

    @Query("UPDATE propiedades SET is_deleted = 1 WHERE id = :id")
    suspend fun markDeleted(id: String)

    @Query("DELETE FROM propiedades")
    suspend fun deleteAll()
}
