package com.propmanager.core.data.repository

import com.propmanager.core.database.dao.GastoDao
import com.propmanager.core.database.dao.SyncQueueDao
import com.propmanager.core.database.entity.GastoEntity
import com.propmanager.core.database.entity.SyncQueueEntry
import com.propmanager.core.database.mapper.toDomain
import com.propmanager.core.database.mapper.toEntity
import com.propmanager.core.model.Gasto
import com.propmanager.core.model.dto.CreateGastoRequest
import com.propmanager.core.model.dto.UpdateGastoRequest
import com.propmanager.core.network.api.GastosApiService
import java.time.Instant
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

@Singleton
open class GastosRepository
@Inject
constructor(
    private val dao: GastoDao,
    private val syncQueueDao: SyncQueueDao,
    private val apiService: GastosApiService,
    private val json: Json,
) {
    open fun observeAll(): Flow<List<Gasto>> =
        dao.observeAll().map { entities -> entities.map { it.toDomain() } }

    open fun observeFiltered(
        propiedadId: String? = null,
        categoria: String? = null,
        estado: String? = null,
        fechaDesde: String? = null,
        fechaHasta: String? = null,
    ): Flow<List<Gasto>> =
        dao.observeFiltered(propiedadId, categoria, estado, fechaDesde, fechaHasta).map { entities
            ->
            entities.map { it.toDomain() }
        }

    open suspend fun create(request: CreateGastoRequest): Result<Gasto> = runCatching {
        val id = UUID.randomUUID().toString()
        val now = Instant.now().toEpochMilli()
        val entity =
            GastoEntity(
                id = id,
                propiedadId = request.propiedadId,
                unidadId = request.unidadId,
                categoria = request.categoria,
                descripcion = request.descripcion,
                monto = request.monto,
                moneda = request.moneda,
                fechaGasto = request.fechaGasto,
                estado = "pendiente",
                proveedor = request.proveedor,
                numeroFactura = request.numeroFactura,
                notas = request.notas,
                createdAt = now,
                updatedAt = now,
                isPendingSync = true,
            )
        dao.upsert(entity)
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "gasto",
                entityId = id,
                operation = "CREATE",
                payload = json.encodeToString(request),
                createdAt = now,
            )
        )
        entity.toDomain()
    }

    open suspend fun update(id: String, request: UpdateGastoRequest): Result<Unit> = runCatching {
        syncQueueDao.enqueue(
            SyncQueueEntry(
                entityType = "gasto",
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
                entityType = "gasto",
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
