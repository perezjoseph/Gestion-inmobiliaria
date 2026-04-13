package com.propmanager.core.data.repository

import com.propmanager.core.database.dao.PagoDao
import com.propmanager.core.database.dao.SyncQueueDao
import com.propmanager.core.database.entity.PagoEntity
import com.propmanager.core.database.entity.SyncQueueEntry
import com.propmanager.core.database.mapper.toDomain
import com.propmanager.core.database.mapper.toEntity
import com.propmanager.core.model.Pago
import com.propmanager.core.model.dto.CreatePagoRequest
import com.propmanager.core.model.dto.UpdatePagoRequest
import com.propmanager.core.network.api.PagosApiService
import java.time.Instant
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

@Singleton
open class PagosRepository
@Inject
constructor(
    private val dao: PagoDao,
    private val syncQueueDao: SyncQueueDao,
    private val apiService: PagosApiService,
    private val json: Json,
) {
    open fun observeAll(): Flow<List<Pago>> =
        dao.observeAll().map { entities -> entities.map { it.toDomain() } }

    open fun observeFiltered(
        contratoId: String? = null,
        estado: String? = null,
        fechaDesde: String? = null,
        fechaHasta: String? = null,
    ): Flow<List<Pago>> =
        dao.observeFiltered(contratoId, estado, fechaDesde, fechaHasta).map { entities ->
            entities.map { it.toDomain() }
        }

    open suspend fun create(request: CreatePagoRequest): Result<Pago> = runCatching {
        val id = UUID.randomUUID().toString()
        val now = Instant.now().toEpochMilli()
        val entity =
            PagoEntity(
                id = id,
                contratoId = request.contratoId,
                monto = request.monto,
                moneda = request.moneda ?: "DOP",
                fechaPago = request.fechaPago,
                fechaVencimiento = request.fechaVencimiento,
                metodoPago = request.metodoPago,
                estado = "pendiente",
                notas = request.notas,
                createdAt = now,
                updatedAt = now,
                isPendingSync = true,
            )
        dao.upsert(entity)
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "pago",
                entityId = id,
                operation = "CREATE",
                payload = json.encodeToString(request),
                createdAt = now,
            )
        )
        entity.toDomain()
    }

    open suspend fun update(id: String, request: UpdatePagoRequest): Result<Unit> = runCatching {
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "pago",
                entityId = id,
                operation = "UPDATE",
                payload = json.encodeToString(request),
                createdAt = Instant.now().toEpochMilli(),
            )
        )
    }

    open suspend fun delete(id: String): Result<Unit> = runCatching {
        dao.markDeleted(id)
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "pago",
                entityId = id,
                operation = "DELETE",
                payload = "",
                createdAt = Instant.now().toEpochMilli(),
            )
        )
    }

    open suspend fun refreshFromServer(): Result<Unit> = runCatching {
        var page = 1L
        do {
            val response = apiService.list(mapOf("page" to page.toString(), "perPage" to "100"))
            val body = response.body() ?: break
            dao.upsertAll(body.data.map { it.toEntity() })
            page++
        } while (body.data.size.toLong() == body.perPage)
    }
}
