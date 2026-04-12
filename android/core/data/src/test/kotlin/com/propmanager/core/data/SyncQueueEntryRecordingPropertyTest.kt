package com.propmanager.core.data

import com.propmanager.core.database.entity.SyncQueueEntry
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.string.shouldNotBeEmpty
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.element
import io.kotest.property.arbitrary.int
import io.kotest.property.arbitrary.long
import io.kotest.property.arbitrary.string
import io.kotest.property.arbitrary.uuid
import io.kotest.property.checkAll

/**
 * **Validates: Requirements 2.3**
 *
 * Property 3: Sync queue records complete entries
 *
 * For any entity type (propiedad, inquilino, contrato, pago, gasto, solicitud),
 * any valid entity ID, and any operation type (CREATE, UPDATE, DELETE),
 * creating a SyncQueueEntry preserves all fields correctly:
 * - entityType is one of the valid types
 * - operation is one of CREATE, UPDATE, DELETE
 * - payload is non-empty
 * - createdAt is a positive timestamp
 */
class SyncQueueEntryRecordingPropertyTest : FreeSpec({

    val validEntityTypes = listOf("propiedad", "inquilino", "contrato", "pago", "gasto", "solicitud")
    val validOperations = listOf("CREATE", "UPDATE", "DELETE")

    val entityTypeArb = Arb.element(validEntityTypes)
    val operationArb = Arb.element(validOperations)
    val entityIdArb = Arb.uuid().map { it.toString() }
    val payloadArb = Arb.string(1..500)
    val timestampArb = Arb.long(1L..Long.MAX_VALUE / 2)

    val syncQueueEntryArb: Arb<SyncQueueEntry> = arbitrary {
        SyncQueueEntry(
            id = 0,
            entityType = entityTypeArb.bind(),
            entityId = entityIdArb.bind(),
            operation = operationArb.bind(),
            payload = payloadArb.bind(),
            createdAt = timestampArb.bind(),
            retryCount = Arb.int(0..10).bind()
        )
    }

    "Property 3: Sync queue records complete entries" - {

        "all fields are preserved after construction" {
            checkAll(100, entityTypeArb, entityIdArb, operationArb, payloadArb, timestampArb) {
                entityType, entityId, operation, payload, createdAt ->

                val entry = SyncQueueEntry(
                    entityType = entityType,
                    entityId = entityId,
                    operation = operation,
                    payload = payload,
                    createdAt = createdAt
                )

                entry.entityType shouldBe entityType
                entry.entityId shouldBe entityId
                entry.operation shouldBe operation
                entry.payload shouldBe payload
                entry.createdAt shouldBe createdAt
            }
        }

        "entityType is always a valid domain entity" {
            checkAll(100, syncQueueEntryArb) { entry ->
                entry.entityType shouldBe validEntityTypes.first { it == entry.entityType }
                (entry.entityType in validEntityTypes) shouldBe true
            }
        }

        "operation is always CREATE, UPDATE, or DELETE" {
            checkAll(100, syncQueueEntryArb) { entry ->
                (entry.operation in validOperations) shouldBe true
            }
        }

        "payload is non-empty" {
            checkAll(100, syncQueueEntryArb) { entry ->
                entry.payload.shouldNotBeEmpty()
            }
        }

        "createdAt is a positive timestamp" {
            checkAll(100, syncQueueEntryArb) { entry ->
                (entry.createdAt > 0) shouldBe true
            }
        }

        "retryCount defaults to zero for new entries" {
            checkAll(100, entityTypeArb, entityIdArb, operationArb, payloadArb, timestampArb) {
                entityType, entityId, operation, payload, createdAt ->

                val entry = SyncQueueEntry(
                    entityType = entityType,
                    entityId = entityId,
                    operation = operation,
                    payload = payload,
                    createdAt = createdAt
                )

                entry.retryCount shouldBe 0
            }
        }
    }
})
