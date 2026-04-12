package com.propmanager.core.data.repository

import com.propmanager.core.database.dao.InquilinoDao
import com.propmanager.core.database.dao.SyncQueueDao
import com.propmanager.core.database.entity.InquilinoEntity
import com.propmanager.core.database.entity.SyncQueueEntry
import com.propmanager.core.database.mapper.toDomain
import com.propmanager.core.database.mapper.toEntity
import com.propmanager.core.model.Inquilino
import com.propmanager.core.model.dto.CreateInquilinoRequest
import com.propmanager.core.model.dto.UpdateInquilinoRequest
import com.propmanager.core.network.api.InquilinosApiService
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.time.Instant
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class InquilinosRepository @Inject constructor(
    private val dao: InquilinoDao,
    private val syncQueueDao: SyncQueueDao,
    private val apiService: InquilinosApiService,
    private val json: Json
) {

    fun observeAll(): Flow<List<Inquilino>> =
        dao.observeAll().map { entities -> entities.map { it.toDomain() } }

    fun search(query: String): Flow<List<Inquilino>> =
        dao.search(query).map { entities -> entities.map { it.toDomain() } }

    fun observeById(id: String): Flow<Inquilino?> =
        dao.observeById(id).map { it?.toDomain() }

    suspend fun create(request: CreateInquilinoRequest): Result<Inquilino> = runCatching {
        val id = UUID.randomUUID().toString()
        val now = Instant.now().toEpochMilli()
        val entity = InquilinoEntity(
            id = id,
            nombre = request.nombre,
            apellido = request.apellido,
            email = request.email,
            telefono = request.telefono,
            cedula = request.cedula,
            contactoEmergencia = request.contactoEmergencia,
            notas = request.notas,
            createdAt = now,
            updatedAt = now,
            isPendingSync = true
        )
        dao.upsert(entity)
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "inquilino",
                entityId = id,
                operation = "CREATE",
                payload = json.encodeToString(request),
                createdAt = now
            )
        )
        entity.toDomain()
    }

    suspend fun update(id: String, request: UpdateInquilinoRequest): Result<Unit> = runCatching {
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "inquilino",
                entityId = id,
                operation = "UPDATE",
                payload = json.encodeToString(request),
                createdAt = Instant.now().toEpochMilli()
            )
        )
    }

    suspend fun delete(id: String): Result<Unit> = runCatching {
        dao.markDeleted(id)
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "inquilino",
                entityId = id,
                operation = "DELETE",
                payload = "",
                createdAt = Instant.now().toEpochMilli()
            )
        )
    }

    suspend fun refreshFromServer(): Result<Unit> = runCatching {
        var page = 1L
        do {
            val response = apiService.list(mapOf("page" to page.toString(), "perPage" to "100"))
            val body = response.body() ?: break
            dao.upsertAll(body.data.map { it.toEntity() })
            page++
        } while (body.data.size.toLong() == body.perPage)
    }
}
