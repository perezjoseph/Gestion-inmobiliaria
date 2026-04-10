package com.propmanager.core.database.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Insert
import androidx.room.Query
import com.propmanager.core.database.entity.SyncQueueEntry
import kotlinx.coroutines.flow.Flow

@Dao
interface SyncQueueDao {
    @Query("SELECT * FROM sync_queue ORDER BY created_at ASC")
    suspend fun getAllPending(): List<SyncQueueEntry>

    @Insert suspend fun enqueue(entry: SyncQueueEntry)

    @Delete suspend fun remove(entry: SyncQueueEntry)

    @Query("SELECT COUNT(*) FROM sync_queue") fun observePendingCount(): Flow<Int>
}
