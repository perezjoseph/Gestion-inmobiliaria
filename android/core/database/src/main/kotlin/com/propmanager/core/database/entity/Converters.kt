package com.propmanager.core.database.entity

import androidx.room.TypeConverter
import java.time.Instant

class Converters {
    @TypeConverter
    fun fromTimestamp(value: Long?): Instant? = value?.let { Instant.ofEpochMilli(it) }

    @TypeConverter fun toTimestamp(instant: Instant?): Long? = instant?.toEpochMilli()
}
