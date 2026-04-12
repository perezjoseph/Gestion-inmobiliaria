package com.propmanager.core.database.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import com.propmanager.core.database.entity.DashboardCache

@Dao
interface DashboardCacheDao {
    @Query("SELECT * FROM dashboard_cache WHERE `key` = :key")
    suspend fun getByKey(key: String): DashboardCache?

    @Upsert
    suspend fun upsert(cache: DashboardCache)

    @Query("DELETE FROM dashboard_cache")
    suspend fun deleteAll()
}
