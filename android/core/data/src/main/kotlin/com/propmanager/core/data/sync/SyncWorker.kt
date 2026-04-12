package com.propmanager.core.data.sync

import android.content.Context
import android.util.Log
import androidx.hilt.work.HiltWorker
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import com.propmanager.core.database.dao.ContratoDao
import com.propmanager.core.database.dao.GastoDao
import com.propmanager.core.database.dao.InquilinoDao
import com.propmanager.core.database.dao.PropiedadDao
import com.propmanager.core.database.dao.SolicitudMantenimientoDao
import com.propmanager.core.database.dao.PagoDao
import com.propmanager.core.database.dao.SyncQueueDao
import com.propmanager.core.database.entity.SyncQueueEntry
import com.propmanager.core.database.mapper.toEntity
import com.propmanager.core.model.dto.CreateContratoRequest
import com.propmanager.core.model.dto.CreateGastoRequest
import com.propmanager.core.model.dto.CreateInquilinoRequest
import com.propmanager.core.model.dto.CreateNotaRequest
import com.propmanager.core.model.dto.CreatePagoRequest
import com.propmanager.core.model.dto.CreatePropiedadRequest
import com.propmanager.core.model.dto.CreateSolicitudRequest
import com.propmanager.core.model.dto.RenovarContratoRequest
import com.propmanager.core.model.dto.TerminarContratoRequest
import com.propmanager.core.model.dto.UpdateEstadoRequest
import com.propmanager.core.model.dto.UpdateGastoRequest
import com.propmanager.core.model.dto.UpdateInquilinoRequest
import com.propmanager.core.model.dto.UpdatePagoRequest
import com.propmanager.core.model.dto.UpdatePropiedadRequest
import com.propmanager.core.model.dto.UpdateSolicitudRequest
import com.propmanager.core.network.api.ContratosApiService
import com.propmanager.core.network.api.GastosApiService
import com.propmanager.core.network.api.InquilinosApiService
import com.propmanager.core.network.api.MantenimientoApiService
import com.propmanager.core.network.api.PagosApiService
import com.propmanager.core.network.api.PropiedadesApiService
import dagger.assisted.Assisted
import dagger.assisted.AssistedInject
import kotlinx.serialization.json.Json
import retrofit2.HttpException
import java.io.IOException

@HiltWorker
class SyncWorker @AssistedInject constructor(
    @Assisted context: Context,
    @Assisted params: WorkerParameters,
    private val syncQueueDao: SyncQueueDao,
    private val propiedadesApi: PropiedadesApiService,
    private val inquilinosApi: InquilinosApiService,
    private val contratosApi: ContratosApiService,
    private val pagosApi: PagosApiService,
    private val gastosApi: GastosApiService,
    private val mantenimientoApi: MantenimientoApiService,
    private val propiedadDao: PropiedadDao,
    private val inquilinoDao: InquilinoDao,
    private val contratoDao: ContratoDao,
    private val pagoDao: PagoDao,
    private val gastoDao: GastoDao,
    private val solicitudDao: SolicitudMantenimientoDao,
    private val json: Json
) : CoroutineWorker(context, params) {

    override suspend fun doWork(): Result {
        val pending = syncQueueDao.getAllPending()
        for (entry in pending) {
            try {
                processEntry(entry)
                syncQueueDao.remove(entry)
            } catch (e: HttpException) {
                if (e.code() == 409) {
                    handleConflict(entry)
                    syncQueueDao.remove(entry)
                } else {
                    Log.e(TAG, "HTTP error syncing ${entry.entityType}/${entry.entityId}: ${e.code()}")
                    return Result.retry()
                }
            } catch (e: IOException) {
                Log.e(TAG, "Network error syncing ${entry.entityType}/${entry.entityId}", e)
                return Result.retry()
            }
        }
        return Result.success()
    }

    private suspend fun processEntry(entry: SyncQueueEntry) {
        when (entry.entityType) {
            "propiedad" -> processPropiedad(entry)
            "inquilino" -> processInquilino(entry)
            "contrato" -> processContrato(entry)
            "pago" -> processPago(entry)
            "gasto" -> processGasto(entry)
            "mantenimiento" -> processMantenimiento(entry)
            "mantenimiento_nota" -> processMantenimientoNota(entry)
        }
    }

    private suspend fun processPropiedad(entry: SyncQueueEntry) {
        when (entry.operation) {
            "CREATE" -> {
                val request = json.decodeFromString<CreatePropiedadRequest>(entry.payload)
                val response = propiedadesApi.create(request)
                response.body()?.let { propiedadDao.upsert(it.toEntity()) }
            }
            "UPDATE" -> {
                val request = json.decodeFromString<UpdatePropiedadRequest>(entry.payload)
                val response = propiedadesApi.update(entry.entityId, request)
                response.body()?.let { propiedadDao.upsert(it.toEntity()) }
            }
            "DELETE" -> propiedadesApi.delete(entry.entityId)
        }
    }

    private suspend fun processInquilino(entry: SyncQueueEntry) {
        when (entry.operation) {
            "CREATE" -> {
                val request = json.decodeFromString<CreateInquilinoRequest>(entry.payload)
                val response = inquilinosApi.create(request)
                response.body()?.let { inquilinoDao.upsert(it.toEntity()) }
            }
            "UPDATE" -> {
                val request = json.decodeFromString<UpdateInquilinoRequest>(entry.payload)
                val response = inquilinosApi.update(entry.entityId, request)
                response.body()?.let { inquilinoDao.upsert(it.toEntity()) }
            }
            "DELETE" -> inquilinosApi.delete(entry.entityId)
        }
    }

    private suspend fun processContrato(entry: SyncQueueEntry) {
        when (entry.operation) {
            "CREATE" -> {
                val request = json.decodeFromString<CreateContratoRequest>(entry.payload)
                val response = contratosApi.create(request)
                response.body()?.let { contratoDao.upsert(it.toEntity()) }
            }
            "RENEW" -> {
                val request = json.decodeFromString<RenovarContratoRequest>(entry.payload)
                val response = contratosApi.renovar(entry.entityId, request)
                response.body()?.let { contratoDao.upsert(it.toEntity()) }
            }
            "TERMINATE" -> {
                val request = json.decodeFromString<TerminarContratoRequest>(entry.payload)
                val response = contratosApi.terminar(entry.entityId, request)
                response.body()?.let { contratoDao.upsert(it.toEntity()) }
            }
            "DELETE" -> contratosApi.delete(entry.entityId)
        }
    }

    private suspend fun processPago(entry: SyncQueueEntry) {
        when (entry.operation) {
            "CREATE" -> {
                val request = json.decodeFromString<CreatePagoRequest>(entry.payload)
                val response = pagosApi.create(request)
                response.body()?.let { pagoDao.upsert(it.toEntity()) }
            }
            "UPDATE" -> {
                val request = json.decodeFromString<UpdatePagoRequest>(entry.payload)
                val response = pagosApi.update(entry.entityId, request)
                response.body()?.let { pagoDao.upsert(it.toEntity()) }
            }
            "DELETE" -> pagosApi.delete(entry.entityId)
        }
    }

    private suspend fun processGasto(entry: SyncQueueEntry) {
        when (entry.operation) {
            "CREATE" -> {
                val request = json.decodeFromString<CreateGastoRequest>(entry.payload)
                val response = gastosApi.create(request)
                response.body()?.let { gastoDao.upsert(it.toEntity()) }
            }
            "UPDATE" -> {
                val request = json.decodeFromString<UpdateGastoRequest>(entry.payload)
                val response = gastosApi.update(entry.entityId, request)
                response.body()?.let { gastoDao.upsert(it.toEntity()) }
            }
            "DELETE" -> gastosApi.delete(entry.entityId)
        }
    }

    private suspend fun processMantenimiento(entry: SyncQueueEntry) {
        when (entry.operation) {
            "CREATE" -> {
                val request = json.decodeFromString<CreateSolicitudRequest>(entry.payload)
                val response = mantenimientoApi.create(request)
                response.body()?.let { solicitudDao.upsert(it.toEntity()) }
            }
            "UPDATE" -> {
                val request = json.decodeFromString<UpdateSolicitudRequest>(entry.payload)
                val response = mantenimientoApi.update(entry.entityId, request)
                response.body()?.let { solicitudDao.upsert(it.toEntity()) }
            }
            "UPDATE_ESTADO" -> {
                val request = json.decodeFromString<UpdateEstadoRequest>(entry.payload)
                val response = mantenimientoApi.updateEstado(entry.entityId, request)
                response.body()?.let { solicitudDao.upsert(it.toEntity()) }
            }
            "DELETE" -> mantenimientoApi.delete(entry.entityId)
        }
    }

    private suspend fun processMantenimientoNota(entry: SyncQueueEntry) {
        val request = json.decodeFromString<CreateNotaRequest>(entry.payload)
        mantenimientoApi.addNota(entry.entityId, request)
    }

    private suspend fun handleConflict(entry: SyncQueueEntry) {
        Log.w(TAG, "Conflict (409) for ${entry.entityType}/${entry.entityId}, applying server-wins")
        when (entry.entityType) {
            "propiedad" -> {
                val response = propiedadesApi.getById(entry.entityId)
                response.body()?.let { propiedadDao.upsert(it.toEntity()) }
            }
            "inquilino" -> {
                val response = inquilinosApi.getById(entry.entityId)
                response.body()?.let { inquilinoDao.upsert(it.toEntity()) }
            }
            "contrato" -> {
                val response = contratosApi.getById(entry.entityId)
                response.body()?.let { contratoDao.upsert(it.toEntity()) }
            }
            "mantenimiento" -> {
                val response = mantenimientoApi.getById(entry.entityId)
                response.body()?.let { solicitudDao.upsert(it.toEntity()) }
            }
        }
    }

    companion object {
        const val TAG = "SyncWorker"
        const val WORK_NAME = "sync_worker"
    }
}
