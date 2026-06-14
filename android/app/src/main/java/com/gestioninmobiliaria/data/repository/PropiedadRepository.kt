package com.gestioninmobiliaria.data.repository

import com.gestioninmobiliaria.data.local.PropiedadDao
import com.gestioninmobiliaria.data.local.toEntity
import com.gestioninmobiliaria.data.local.toModel
import com.gestioninmobiliaria.data.model.Propiedad
import com.gestioninmobiliaria.data.remote.PropiedadApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import timber.log.Timber
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class PropiedadRepository @Inject constructor(
    private val api: PropiedadApi,
    private val dao: PropiedadDao,
) {
    fun observePropiedades(): Flow<List<Propiedad>> =
        dao.getAll().map { entities -> entities.map { it.toModel() } }

    suspend fun refresh() {
        try {
            val remote = api.getPropiedades()
            dao.insertAll(remote.map { it.toEntity() })
            dao.deleteNotIn(remote.map { it.id })
        } catch (e: Exception) {
            Timber.e(e, "Error syncing propiedades")
            throw e
        }
    }
}
