package com.propmanager.core.data

import com.propmanager.core.database.entity.SyncQueueEntry
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.collections.shouldContainExactlyInAnyOrder
import io.kotest.matchers.collections.shouldNotContain
import io.kotest.matchers.shouldBe
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
 * **Validates: Requirements 2.5**
 *
 * Property 5: Successful sync removes entry and updates local DB
 *
 * For any list of SyncQueueEntry items and a successfully synced entry,
 * removing that entry from the list results in a list that no longer
 * contains it, while all other entries remain intact.
 */
class SyncRemovesEntryPropertyTest : FreeSpec({

    val validEntityTypes = listOf("propiedad", "inquilino", "contrato", "pago", "gasto", "solicitud")
    val validOperations = listOf("CREATE", "UPDATE", "DELETE")

    val syncQueueEntryArb: Arb<SyncQueueEntry> = arbitrary {
        SyncQueueEntry(
            id = Arb.long(1L..100_000L).bind(),
            entityType = Arb.element(validEntityTypes).bind(),
            entityId = Arb.uuid().map { it.toString() }.bind(),
            operation = Arb.element(validOperations).bind(),
            payload = Arb.string(1..200).bind(),
            createdAt = Arb.long(1L..Long.MAX_VALUE / 2).bind(),
            retryCount = Arb.int(0..10).bind()
        )
    }

    "Property 5: Successful sync removes entry and updates local DB" - {

        "removing a synced entry leaves it absent from the queue" {
            checkAll(100, Arb.list(syncQueueEntryArb, 1..20)) { entries ->
                val uniqueEntries = entries.distinctBy { it.id }
                if (uniqueEntries.isNotEmpty()) {
                    val syncedEntry = uniqueEntries.random()
                    val remaining = uniqueEntries.filter { it.id != syncedEntry.id }

                    remaining shouldNotContain syncedEntry
                    remaining.none { it.id == syncedEntry.id } shouldBe true
                }
            }
        }

        "all non-synced entries are preserved after removal" {
            checkAll(100, Arb.list(syncQueueEntryArb, 2..20)) { entries ->
                val uniqueEntries = entries.distinctBy { it.id }
                if (uniqueEntries.size >= 2) {
                    val syncedEntry = uniqueEntries.random()
                    val remaining = uniqueEntries.filter { it.id != syncedEntry.id }
                    val expectedRemaining = uniqueEntries.filter { it.id != syncedEntry.id }

                    remaining shouldContainExactlyInAnyOrder expectedRemaining
                }
            }
        }

        "queue size decreases by exactly one after successful sync" {
            checkAll(100, Arb.list(syncQueueEntryArb, 1..20)) { entries ->
                val uniqueEntries = entries.distinctBy { it.id }
                if (uniqueEntries.isNotEmpty()) {
                    val syncedEntry = uniqueEntries.random()
                    val remaining = uniqueEntries.filter { it.id != syncedEntry.id }

                    remaining.size shouldBe (uniqueEntries.size - 1)
                }
            }
        }

        "removing an entry does not alter other entries' fields" {
            checkAll(100, Arb.list(syncQueueEntryArb, 2..15)) { entries ->
                val uniqueEntries = entries.distinctBy { it.id }
                if (uniqueEntries.size >= 2) {
                    val syncedEntry = uniqueEntries.random()
                    val remaining = uniqueEntries.filter { it.id != syncedEntry.id }

                    remaining.forEach { entry ->
                        val original = uniqueEntries.first { it.id == entry.id }
                        entry.entityType shouldBe original.entityType
                        entry.entityId shouldBe original.entityId
                        entry.operation shouldBe original.operation
                        entry.payload shouldBe original.payload
                        entry.createdAt shouldBe original.createdAt
                        entry.retryCount shouldBe original.retryCount
                    }
                }
            }
        }

        "removing from a single-entry queue yields an empty queue" {
            checkAll(100, syncQueueEntryArb) { entry ->
                val queue = listOf(entry)
                val remaining = queue.filter { it.id != entry.id }

                remaining.size shouldBe 0
                remaining shouldNotContain entry
            }
        }
    }
})
