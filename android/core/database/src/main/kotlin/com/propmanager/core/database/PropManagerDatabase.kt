package com.propmanager.core.database

import androidx.room.Database
import androidx.room.RoomDatabase
import androidx.room.TypeConverters
import com.propmanager.core.database.dao.ContratoDao
import com.propmanager.core.database.dao.DashboardCacheDao
import com.propmanager.core.database.dao.GastoDao
import com.propmanager.core.database.dao.InquilinoDao
import com.propmanager.core.database.dao.NotaMantenimientoDao
import com.propmanager.core.database.dao.PagoDao
import com.propmanager.core.database.dao.PropiedadDao
import com.propmanager.core.database.dao.SolicitudMantenimientoDao
import com.propmanager.core.database.dao.SyncQueueDao
import com.propmanager.core.database.entity.ContratoEntity
import com.propmanager.core.database.entity.Converters
import com.propmanager.core.database.entity.DashboardCache
import com.propmanager.core.database.entity.GastoEntity
import com.propmanager.core.database.entity.InquilinoEntity
import com.propmanager.core.database.entity.NotaMantenimientoEntity
import com.propmanager.core.database.entity.PagoEntity
import com.propmanager.core.database.entity.PropiedadEntity
import com.propmanager.core.database.entity.SolicitudMantenimientoEntity
import com.propmanager.core.database.entity.SyncQueueEntry

@Database(
    entities = [
        PropiedadEntity::class,
        InquilinoEntity::class,
        ContratoEntity::class,
        PagoEntity::class,
        GastoEntity::class,
        SolicitudMantenimientoEntity::class,
        NotaMantenimientoEntity::class,
        SyncQueueEntry::class,
        DashboardCache::class,
    ],
    version = 1,
    exportSchema = true,
)
@TypeConverters(Converters::class)
abstract class PropManagerDatabase : RoomDatabase() {
    abstract fun propiedadDao(): PropiedadDao

    abstract fun inquilinoDao(): InquilinoDao

    abstract fun contratoDao(): ContratoDao

    abstract fun pagoDao(): PagoDao

    abstract fun gastoDao(): GastoDao

    abstract fun solicitudDao(): SolicitudMantenimientoDao

    abstract fun notaDao(): NotaMantenimientoDao

    abstract fun syncQueueDao(): SyncQueueDao

    abstract fun dashboardCacheDao(): DashboardCacheDao
}
