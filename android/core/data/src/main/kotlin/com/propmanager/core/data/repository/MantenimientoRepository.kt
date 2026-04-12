package com.propmanager.core.data.repository

import com.propmanager.core.database.dao.NotaMantenimientoDao
import com.propmanager.core.database.dao.SolicitudMantenimientoDao
import com.propmanager.core.database.dao.SyncQueueDao
import com.propmanager.core.database.entity.NotaMantenimientoEntity
import com.propmanager.core.database.entity.SolicitudMantenimientoEntity
import com.propmanager.core.database.entity.SyncQueueEntry
import com.propmanager.core.database.mapper.toDomain
import com.propmanager.core.database.mapper.toEntity
import com.propmanager.core.model.NotaMantenimiento
import com.propmanager.core.model.SolicitudMantenimiento
import com.propmanager.core.model.dto.CreateNotaRequest
import com.propmanager.core.model.dto.CreateSolicitudRequest
import com.propmanager.core.model.dto.UpdateEstadoRequest
import com.propmanager.core.model.dto.UpdateSolicitudRequest
import com.propmanager.core.network.api.MantenimientoApiService
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.time.Instant
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class MantenimientoRepository
    @Inject
    constructor(
        private val solicitudDao: SolicitudMantenimientoDao,
        private val notaDao: NotaMantenimientoDao,
        private val syncQueueDao: SyncQueueDao,
        private val apiService: MantenimientoApiService,
        private val json: Json,
    ) {
        fun observeAll(): Flow<List<SolicitudMantenimiento>> = solicitudDao.observeAll().map { entities -> entities.map { it.toDomain() } }

        fun observeFiltered(
            estado: String? = null,
            prioridad: String? = null,
            propiedadId: String? = null,
        ): Flow<List<SolicitudMantenimiento>> =
            solicitudDao
                .observeFiltered(estado, prioridad, propiedadId)
                .map { entities -> entities.map { it.toDomain() } }

        fun observeById(id: String): Flow<SolicitudMantenimiento?> = solicitudDao.observeById(id).map { it?.toDomain() }

        fun observeNotas(solicitudId: String): Flow<List<NotaMantenimiento>> =
            notaDao.observeBySolicitudId(solicitudId).map { entities -> entities.map { it.toDomain() } }

        suspend fun create(request: CreateSolicitudRequest): Result<SolicitudMantenimiento> =
            runCatching {
                val id = UUID.randomUUID().toString()
                val now = Instant.now().toEpochMilli()
                val entity =
                    SolicitudMantenimientoEntity(
                        id = id,
                        propiedadId = request.propiedadId,
                        unidadId = request.unidadId,
                        inquilinoId = request.inquilinoId,
                        titulo = request.titulo,
                        descripcion = request.descripcion,
                        estado = "pendiente",
                        prioridad = request.prioridad ?: "media",
                        nombreProveedor = request.nombreProveedor,
                        telefonoProveedor = request.telefonoProveedor,
                        emailProveedor = request.emailProveedor,
                        costoMonto = request.costoMonto,
                        costoMoneda = request.costoMoneda,
                        fechaInicio = null,
                        fechaFin = null,
                        createdAt = now,
                        updatedAt = now,
                        isPendingSync = true,
                    )
                solicitudDao.upsert(entity)
                syncQueueDao.enqueue(
                    SyncQueueEntry(
                        entityType = "mantenimiento",
                        entityId = id,
                        operation = "CREATE",
                        payload = json.encodeToString(request),
                        createdAt = now,
                    ),
                )
                entity.toDomain()
            }

        suspend fun update(
            id: String,
            request: UpdateSolicitudRequest,
        ): Result<Unit> =
            runCatching {
                syncQueueDao.enqueue(
                    SyncQueueEntry(
                        entityType = "mantenimiento",
                        entityId = id,
                        operation = "UPDATE",
                        payload = json.encodeToString(request),
                        createdAt = Instant.now().toEpochMilli(),
                    ),
                )
            }

        suspend fun updateEstado(
            id: String,
            request: UpdateEstadoRequest,
        ): Result<Unit> =
            runCatching {
                syncQueueDao.enqueue(
                    SyncQueueEntry(
                        entityType = "mantenimiento",
                        entityId = id,
                        operation = "UPDATE_ESTADO",
                        payload = json.encodeToString(request),
                        createdAt = Instant.now().toEpochMilli(),
                    ),
                )
            }

        suspend fun addNota(
            solicitudId: String,
            request: CreateNotaRequest,
        ): Result<NotaMantenimiento> =
            runCatching {
                val id = UUID.randomUUID().toString()
                val now = Instant.now().toEpochMilli()
                val entity =
                    NotaMantenimientoEntity(
                        id = id,
                        solicitudId = solicitudId,
                        autorId = "",
                        contenido = request.contenido,
                        createdAt = now,
                    )
                notaDao.insert(entity)
                syncQueueDao.enqueue(
                    SyncQueueEntry(
                        entityType = "mantenimiento_nota",
                        entityId = solicitudId,
                        operation = "ADD_NOTA",
                        payload = json.encodeToString(request),
                        createdAt = now,
                    ),
                )
                entity.toDomain()
            }

        suspend fun delete(id: String): Result<Unit> =
            runCatching {
                solicitudDao.markDeleted(id)
                syncQueueDao.enqueue(
                    SyncQueueEntry(
                        entityType = "mantenimiento",
                        entityId = id,
                        operation = "DELETE",
                        payload = "",
                        createdAt = Instant.now().toEpochMilli(),
                    ),
                )
            }

        suspend fun refreshFromServer(): Result<Unit> =
            runCatching {
                var page = 1L
                do {
                    val response = apiService.list(mapOf("page" to page.toString(), "perPage" to "100"))
                    val body = response.body() ?: break
                    solicitudDao.upsertAll(body.data.map { it.toEntity() })
                    body.data.forEach { dto ->
                        dto.notas?.forEach { nota -> notaDao.insert(nota.toEntity()) }
                    }
                    page++
                } while (body.data.size.toLong() == body.perPage)
            }
    }
