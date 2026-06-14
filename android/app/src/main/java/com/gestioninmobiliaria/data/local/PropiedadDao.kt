package com.gestioninmobiliaria.data.local

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

@Dao
interface PropiedadDao {
    @Query("SELECT * FROM propiedades ORDER BY updated_at DESC")
    fun getAll(): Flow<List<PropiedadEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertAll(propiedades: List<PropiedadEntity>)

    @Query("DELETE FROM propiedades WHERE id NOT IN (:ids)")
    suspend fun deleteNotIn(ids: List<String>)
}
