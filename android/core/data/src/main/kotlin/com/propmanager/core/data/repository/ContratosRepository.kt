package com.propmanager.core.data.repository

import com.propmanager.core.database.dao.ContratoDao
import com.propmanager.core.database.dao.SyncQueueDao
import com.propmanager.core.database.entity.ContratoEntity
import com.propmanager.core.database.entity.SyncQueueEntry
import com.propmanager.core.database.mapper.toDomain
import com.propmanager.core.database.mapper.toEntity
import com.propmanager.core.model.Contrato
import com.propmanager.core.model.dto.CreateContratoRequest
import com.propmanager.core.model.dto.RenovarContratoRequest
import com.propmanager.core.model.dto.TerminarContratoRequest
import com.propmanager.core.network.api.ContratosApiService
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.time.Instant
import java.time.LocalDate
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ContratosRepository @Inject constructor(
    private val dao: ContratoDao,
    private val syncQueueDao: SyncQueueDao,
    private val apiService: ContratosApiService,
    private val json: Json
) {

    fun observeAll(): Flow<List<Contrato>> =
        dao.observeAll().map { entities -> entities.map { it.toDomain() } }

    fun observeById(id: String): Flow<Contrato?> =
        dao.observeById(id).map { it?.toDomain() }

    fun observeExpiring(daysThreshold: Int = 30): Flow<List<Contrato>> {
        val today = LocalDate.now().toString()
        val threshold = LocalDate.now().plusDays(daysThreshold.toLong()).toString()
        return dao.observeExpiring(today, threshold)
            .map { entities -> entities.map { it.toDomain() } }
    }

    suspend fun create(request: CreateContratoRequest): Result<Contrato> = runCatching {
        val id = UUID.randomUUID().toString()
        val now = Instant.now().toEpochMilli()
        val entity = ContratoEntity(
            id = id,
            propiedadId = request.propiedadId,
            inquilinoId = request.inquilinoId,
            fechaInicio = request.fechaInicio,
            fechaFin = request.fechaFin,
            montoMensual = request.montoMensual,
            deposito = request.deposito,
            moneda = request.moneda ?: "DOP",
            estado = "activo",
            createdAt = now,
            updatedAt = now,
            isPendingSync = true
        )
        dao.upsert(entity)
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "contrato",
                entityId = id,
                operation = "CREATE",
                payload = json.encodeToString(request),
                createdAt = now
            )
        )
        entity.toDomain()
    }

    suspend fun renew(id: String, request: RenovarContratoRequest): Result<Unit> = runCatching {
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "contrato",
                entityId = id,
                operation = "RENEW",
                payload = json.encodeToString(request),
                createdAt = Instant.now().toEpochMilli()
            )
        )
    }

    suspend fun terminate(id: String, request: TerminarContratoRequest): Result<Unit> = runCatching {
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "contrato",
                entityId = id,
                operation = "TERMINATE",
                payload = json.encodeToString(request),
                createdAt = Instant.now().toEpochMilli()
            )
        )
    }

    suspend fun delete(id: String): Result<Unit> = runCatching {
        dao.markDeleted(id)
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "contrato",
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
