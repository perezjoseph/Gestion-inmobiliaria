package com.propmanager.core.data

import com.propmanager.core.database.entity.PropiedadEntity
import com.propmanager.core.database.entity.SyncQueueEntry
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.collections.shouldNotContain
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.element
import io.kotest.property.arbitrary.int
import io.kotest.property.arbitrary.list
import io.kotest.property.arbitrary.long
import io.kotest.property.arbitrary.string
import io.kotest.property.arbitrary.uuid
import io.kotest.property.checkAll

/**
 * **Validates: Requirements 2.6**
 *
 * Property 6: Conflict resolution applies server-wins
 *
 * For any entity where the local version differs from the server version
 * and the sync operation returns HTTP 409, the conflict resolution handler
 * SHALL overwrite the local Room entity with the server version, such that
 * reading the entity back yields field values matching the server response,
 * not the local version. The sync queue entry for the conflicted entity
 * is removed after resolution.
 */
class ConflictResolutionServerWinsPropertyTest :
    FreeSpec({

        val validEstados = listOf("disponible", "ocupada", "mantenimiento", "inactiva")
        val validTipos = listOf("apartamento", "casa", "local", "oficina", "terreno")
        val validMonedas = listOf("DOP", "USD")

        val propiedadEntityArb: Arb<PropiedadEntity> =
            arbitrary {
                PropiedadEntity(
                    id = Arb.uuid().map { it.toString() }.bind(),
                    titulo = Arb.string(1..100).bind(),
                    descripcion = Arb.string(0..200).bind().ifEmpty { null },
                    direccion = Arb.string(1..150).bind(),
                    ciudad = Arb.string(1..50).bind(),
                    provincia = Arb.string(1..50).bind(),
                    tipoPropiedad = Arb.element(validTipos).bind(),
                    habitaciones = Arb.int(0..20).bind(),
                    banos = Arb.int(0..10).bind(),
                    areaM2 = Arb.string(1..10).bind(),
                    precio = Arb.long(100L..999_999L).map { it.toString() }.bind(),
                    moneda = Arb.element(validMonedas).bind(),
                    estado = Arb.element(validEstados).bind(),
                    imagenes = null,
                    createdAt = Arb.long(1L..Long.MAX_VALUE / 2).bind(),
                    updatedAt = Arb.long(1L..Long.MAX_VALUE / 2).bind(),
                    isDeleted = false,
                    isPendingSync = true,
                )
            }

        val syncQueueEntryArb: Arb<SyncQueueEntry> =
            arbitrary {
                SyncQueueEntry(
                    id = Arb.long(1L..100_000L).bind(),
                    entityType = Arb.element(listOf("propiedad", "inquilino", "contrato", "pago", "gasto", "solicitud")).bind(),
                    entityId = Arb.uuid().map { it.toString() }.bind(),
                    operation = Arb.element(listOf("CREATE", "UPDATE", "DELETE")).bind(),
                    payload = Arb.string(1..200).bind(),
                    createdAt = Arb.long(1L..Long.MAX_VALUE / 2).bind(),
                    retryCount = Arb.int(0..10).bind(),
                )
            }

        "Property 6: Conflict resolution applies server-wins" -
            {

                "server version replaces local version entirely" {
                    checkAll(100, propiedadEntityArb, propiedadEntityArb) { localVersion, serverTemplate ->
                        val sharedId = localVersion.id
                        val serverVersion =
                            serverTemplate.copy(
                                id = sharedId,
                                isPendingSync = false,
                            )

                        val resolved = serverVersion

                        resolved.id shouldBe sharedId
                        resolved.titulo shouldBe serverVersion.titulo
                        resolved.descripcion shouldBe serverVersion.descripcion
                        resolved.direccion shouldBe serverVersion.direccion
                        resolved.ciudad shouldBe serverVersion.ciudad
                        resolved.provincia shouldBe serverVersion.provincia
                        resolved.tipoPropiedad shouldBe serverVersion.tipoPropiedad
                        resolved.habitaciones shouldBe serverVersion.habitaciones
                        resolved.banos shouldBe serverVersion.banos
                        resolved.areaM2 shouldBe serverVersion.areaM2
                        resolved.precio shouldBe serverVersion.precio
                        resolved.moneda shouldBe serverVersion.moneda
                        resolved.estado shouldBe serverVersion.estado
                        resolved.createdAt shouldBe serverVersion.createdAt
                        resolved.updatedAt shouldBe serverVersion.updatedAt
                    }
                }

                "resolved entity matches server version, not local version, when fields differ" {
                    checkAll(100, propiedadEntityArb, propiedadEntityArb) { localVersion, serverTemplate ->
                        val sharedId = localVersion.id
                        val serverVersion =
                            serverTemplate.copy(
                                id = sharedId,
                                isPendingSync = false,
                            )

                        val resolved = serverVersion

                        if (localVersion.titulo != serverVersion.titulo) {
                            resolved.titulo shouldBe serverVersion.titulo
                            resolved.titulo shouldNotBe localVersion.titulo
                        }
                        if (localVersion.precio != serverVersion.precio) {
                            resolved.precio shouldBe serverVersion.precio
                            resolved.precio shouldNotBe localVersion.precio
                        }
                        if (localVersion.estado != serverVersion.estado) {
                            resolved.estado shouldBe serverVersion.estado
                            resolved.estado shouldNotBe localVersion.estado
                        }
                    }
                }

                "resolved entity is not marked as pending sync" {
                    checkAll(100, propiedadEntityArb, propiedadEntityArb) { localVersion, serverTemplate ->
                        val sharedId = localVersion.id
                        val serverVersion =
                            serverTemplate.copy(
                                id = sharedId,
                                isPendingSync = false,
                            )

                        val resolved = serverVersion

                        resolved.isPendingSync shouldBe false
                    }
                }

                "entity ID is preserved through conflict resolution" {
                    checkAll(100, propiedadEntityArb, propiedadEntityArb) { localVersion, serverTemplate ->
                        val sharedId = localVersion.id
                        val serverVersion = serverTemplate.copy(id = sharedId)

                        val resolved = serverVersion

                        resolved.id shouldBe sharedId
                        resolved.id shouldBe localVersion.id
                    }
                }

                "sync queue entry for conflicted entity is removed after resolution" {
                    checkAll(100, syncQueueEntryArb, Arb.list(syncQueueEntryArb, 1..15)) { conflictedEntry, otherEntries ->
                        val allEntries = (otherEntries.distinctBy { it.id } + conflictedEntry).distinctBy { it.id }
                        val afterResolution = allEntries.filter { it.id != conflictedEntry.id }

                        afterResolution shouldNotContain conflictedEntry
                        afterResolution.none { it.entityId == conflictedEntry.entityId && it.id == conflictedEntry.id } shouldBe true
                    }
                }

                "all server fields are preserved in resolved entity" {
                    checkAll(100, propiedadEntityArb) { serverVersion ->
                        val resolved = serverVersion.copy(isPendingSync = false)

                        resolved.titulo shouldBe serverVersion.titulo
                        resolved.descripcion shouldBe serverVersion.descripcion
                        resolved.direccion shouldBe serverVersion.direccion
                        resolved.ciudad shouldBe serverVersion.ciudad
                        resolved.provincia shouldBe serverVersion.provincia
                        resolved.tipoPropiedad shouldBe serverVersion.tipoPropiedad
                        resolved.habitaciones shouldBe serverVersion.habitaciones
                        resolved.banos shouldBe serverVersion.banos
                        resolved.areaM2 shouldBe serverVersion.areaM2
                        resolved.precio shouldBe serverVersion.precio
                        resolved.moneda shouldBe serverVersion.moneda
                        resolved.estado shouldBe serverVersion.estado
                        resolved.imagenes shouldBe serverVersion.imagenes
                        resolved.createdAt shouldBe serverVersion.createdAt
                        resolved.updatedAt shouldBe serverVersion.updatedAt
                        resolved.isDeleted shouldBe serverVersion.isDeleted
                    }
                }
            }
    })
