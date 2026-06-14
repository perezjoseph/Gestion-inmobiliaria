package com.gestioninmobiliaria.di

import android.content.Context
import androidx.room.Room
import com.gestioninmobiliaria.data.local.AppDatabase
import com.gestioninmobiliaria.data.local.PropiedadDao
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object DatabaseModule {
    @Provides
    @Singleton
    fun provideDatabase(@ApplicationContext context: Context): AppDatabase =
        Room.databaseBuilder(context, AppDatabase::class.java, "gestion_inmobiliaria.db").build()

    @Provides
    fun providePropiedadDao(db: AppDatabase): PropiedadDao = db.propiedadDao()
}
