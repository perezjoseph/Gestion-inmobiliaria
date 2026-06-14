package com.gestioninmobiliaria.data.local

import androidx.room.Database
import androidx.room.RoomDatabase

@Database(entities = [PropiedadEntity::class], version = 1)
abstract class AppDatabase : RoomDatabase() {
    abstract fun propiedadDao(): PropiedadDao
}
