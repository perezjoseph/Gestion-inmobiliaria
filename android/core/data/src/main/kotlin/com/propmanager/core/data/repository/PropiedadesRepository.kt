package com.propmanager.core.data.repository

import com.propmanager.core.database.dao.PropiedadDao
import com.propmanager.core.database.dao.SyncQueueDao
import com.propmanager.core.database.entity.PropiedadEntity
import com.propmanager.core.database.entity.SyncQueueEntry
import com.propmanager.core.database.mapper.toDomain
import com.propmanager.core.database.mapper.toEntity
import com.propmanager.core.model.Propiedad
import com.propmanager.core.model.dto.CreatePropiedadRequest
import com.propmanager.core.model.dto.UpdatePropiedadRequest
import com.propmanager.core.network.api.PropiedadesApiService
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.time.Instant
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
open class PropiedadesRepository
    @Inject
    constructor(
        private val dao: PropiedadDao,
        private val syncQueueDao: SyncQueueDao,
        private val apiService: PropiedadesApiService,
        private val json: Json,
    ) {
        open fun observeAll(): Flow<List<Propiedad>> = dao.observeAll().map { entities -> entities.map { it.toDomain() } }

        open fun observeFiltered(
            ciudad: String? = null,
            estado: String? = null,
            tipoPropiedad: String? = null,
        ): Flow<List<Propiedad>> =
            dao
                .observeFiltered(ciudad, estado, tipoPropiedad)
                .map { entities -> entities.map { it.toDomain() } }

        open fun observeById(id: String): Flow<Propiedad?> = dao.observeById(id).map { it?.toDomain() }

        open suspend fun create(request: CreatePropiedadRequest): Result<Propiedad> =
            runCatching {
                val id = UUID.randomUUID().toString()
                val now = Instant.now().toEpochMilli()
                val entity =
                    PropiedadEntity(
                        id = id,
                        titulo = request.titulo,
                        descripcion = request.descripcion,
                        direccion = request.direccion,
                        ciudad = request.ciudad,
                        provincia = request.provincia,
                        tipoPropiedad = request.tipoPropiedad,
                        habitaciones = request.habitaciones,
                        banos = request.banos,
                        areaM2 = request.areaM2,
                        precio = request.precio,
                        moneda = request.moneda ?: "DOP",
                        estado = request.estado ?: "disponible",
                        imagenes = request.imagenes?.toString(),
                        createdAt = now,
                        updatedAt = now,
                        isPendingSync = true,
                    )
                dao.upsert(entity)
                syncQueueDao.enqueue(
                    SyncQueueEntry(
                        entityType = "propiedad",
                        entityId = id,
                        operation = "CREATE",
                        payload = json.encodeToString(request),
                        createdAt = now,
                    ),
                )
                entity.toDomain()
            }

        open suspend fun update(
            id: String,
            request: UpdatePropiedadRequest,
        ): Result<Unit> =
            runCatching {
                val now = Instant.now().toEpochMilli()
                syncQueueDao.enqueue(
                    SyncQueueEntry(
                        entityType = "propiedad",
                        entityId = id,
                        operation = "UPDATE",
                        payload = json.encodeToString(request),
                        createdAt = now,
                    ),
                )
            }

        open suspend fun delete(id: String): Result<Unit> =
            runCatching {
                dao.markDeleted(id)
                syncQueueDao.enqueue(
                    SyncQueueEntry(
                        entityType = "propiedad",
                        entityId = id,
                        operation = "DELETE",
                        payload = "",
                        createdAt = Instant.now().toEpochMilli(),
                    ),
                )
            }

        open suspend fun refreshFromServer(): Result<Unit> =
            runCatching {
                var page = 1L
                do {
                    val response = apiService.list(mapOf("page" to page.toString(), "perPage" to "100"))
                    val body = response.body() ?: break
                    dao.upsertAll(body.data.map { it.toEntity() })
                    page++
                } while (body.data.size.toLong() == body.perPage)
            }
    }
