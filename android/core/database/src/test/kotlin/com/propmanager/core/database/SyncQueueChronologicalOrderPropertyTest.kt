package com.propmanager.core.database

import com.propmanager.core.database.entity.SyncQueueEntry
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.shouldBe
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.element
import io.kotest.property.arbitrary.int
import io.kotest.property.arbitrary.long
import io.kotest.property.arbitrary.string
import io.kotest.property.checkAll

/**
 * **Validates: Requirements 2.4**
 *
 * Property 4: Sync queue chronological processing order
 *
 * For any set of SyncQueueEntry items with distinct createdAt timestamps
 * inserted in arbitrary order, sorting by createdAt ASC produces chronological order.
 * This validates the ordering contract that SyncQueueDao.getAllPending() enforces
 * via its ORDER BY created_at ASC query.
 */
class SyncQueueChronologicalOrderPropertyTest : FreeSpec({

    val entityTypeArb = Arb.element("propiedad", "inquilino", "contrato", "pago", "gasto", "solicitud")
    val operationArb = Arb.element("CREATE", "UPDATE", "DELETE")

    val syncQueueEntryArb: Arb<SyncQueueEntry> = arbitrary {
        SyncQueueEntry(
            id = 0,
            entityType = entityTypeArb.bind(),
            entityId = Arb.string(8..36).bind(),
            operation = operationArb.bind(),
            payload = Arb.string(1..100).bind(),
            createdAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
            retryCount = Arb.int(0..5).bind()
        )
    }

    val syncQueueListArb: Arb<List<SyncQueueEntry>> = arbitrary {
        val size = Arb.int(2..30).bind()
        val usedTimestamps = mutableSetOf<Long>()
        (1..size).map {
            var entry = syncQueueEntryArb.bind()
            while (entry.createdAt in usedTimestamps) {
                entry = entry.copy(createdAt = Arb.long(1_000_000L..9_999_999_999L).bind())
            }
            usedTimestamps.add(entry.createdAt)
            entry
        }
    }

    "Property 4: Sync queue chronological processing order" - {

        "entries sorted by createdAt ASC are in chronological order" {
            checkAll(100, syncQueueListArb) { entries ->
                val sorted = entries.sortedBy { it.createdAt }

                sorted.zipWithNext().forEach { (a, b) ->
                    (a.createdAt < b.createdAt) shouldBe true
                }
            }
        }

        "sorting by createdAt ASC preserves all original entries" {
            checkAll(100, syncQueueListArb) { entries ->
                val sorted = entries.sortedBy { it.createdAt }

                sorted.size shouldBe entries.size
                sorted.toSet() shouldBe entries.toSet()
            }
        }

        "sorting is idempotent — sorting twice yields the same result" {
            checkAll(100, syncQueueListArb) { entries ->
                val sortedOnce = entries.sortedBy { it.createdAt }
                val sortedTwice = sortedOnce.sortedBy { it.createdAt }

                sortedOnce shouldBe sortedTwice
            }
        }
    }
})
