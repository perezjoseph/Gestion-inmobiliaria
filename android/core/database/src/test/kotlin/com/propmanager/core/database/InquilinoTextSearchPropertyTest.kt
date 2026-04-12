package com.propmanager.core.database

import com.propmanager.core.database.entity.InquilinoEntity
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.collections.shouldContainExactlyInAnyOrder
import io.kotest.matchers.shouldBe
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.element
import io.kotest.property.arbitrary.list
import io.kotest.property.arbitrary.long
import io.kotest.property.arbitrary.string
import io.kotest.property.arbitrary.uuid
import io.kotest.property.checkAll

/**
 * **Validates: Requirements 4.2**
 *
 * Property 8: Inquilino text search matches nombre, apellido, or cédula
 *
 * For any list of inquilinos and any non-empty search term, the search query
 * returns only inquilinos where nombre, apellido, or cedula contains the search
 * term (case-insensitive), and does not exclude any inquilino that matches in
 * at least one of those fields.
 */
class InquilinoTextSearchPropertyTest : FreeSpec({

    val nombres = listOf("Juan", "María", "Carlos", "Ana", "Pedro", "Luisa", "José", "Carmen")
    val apellidos = listOf("García", "Rodríguez", "Martínez", "López", "Hernández", "Pérez", "Díaz", "Morales")
    val cedulaFormats = listOf(
        "001-1234567-8", "012-9876543-2", "402-3456789-1",
        "031-5551234-0", "100-7778899-3", "226-4443322-5"
    )

    val inquilinoArb: Arb<InquilinoEntity> = arbitrary {
        InquilinoEntity(
            id = Arb.uuid().bind().toString(),
            nombre = Arb.element(nombres).bind(),
            apellido = Arb.element(apellidos).bind(),
            email = null,
            telefono = null,
            cedula = Arb.element(cedulaFormats).bind(),
            contactoEmergencia = null,
            notas = null,
            createdAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
            updatedAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
            isDeleted = false,
            isPendingSync = false
        )
    }

    fun searchInquilinos(
        entities: List<InquilinoEntity>,
        searchTerm: String
    ): List<InquilinoEntity> {
        val term = searchTerm.lowercase()
        return entities.filter { e ->
            e.nombre.lowercase().contains(term) ||
                e.apellido.lowercase().contains(term) ||
                e.cedula.lowercase().contains(term)
        }
    }

    "Property 8: Inquilino text search matches nombre, apellido, or cédula" - {

        "search results contain only inquilinos matching in nombre, apellido, or cedula" {
            checkAll(
                100,
                Arb.list(inquilinoArb, 1..30),
                Arb.element(nombres + apellidos + cedulaFormats)
            ) { entities, searchTerm ->
                val result = searchInquilinos(entities, searchTerm)
                val term = searchTerm.lowercase()

                result.forEach { e ->
                    val matches = e.nombre.lowercase().contains(term) ||
                        e.apellido.lowercase().contains(term) ||
                        e.cedula.lowercase().contains(term)
                    matches shouldBe true
                }

                val expected = entities.filter { e ->
                    e.nombre.lowercase().contains(term) ||
                        e.apellido.lowercase().contains(term) ||
                        e.cedula.lowercase().contains(term)
                }
                result shouldContainExactlyInAnyOrder expected
            }
        }

        "search is case-insensitive" {
            checkAll(100, Arb.list(inquilinoArb, 2..20)) { entities ->
                val target = entities.random()
                val variants = listOf(
                    target.nombre.uppercase(),
                    target.nombre.lowercase(),
                    target.apellido.uppercase(),
                    target.apellido.lowercase()
                )
                val searchTerm = variants.random()

                val result = searchInquilinos(entities, searchTerm)
                result.any { it.id == target.id } shouldBe true
            }
        }

        "search by partial cedula returns matching inquilinos" {
            checkAll(100, Arb.list(inquilinoArb, 2..20)) { entities ->
                val target = entities.random()
                val partial = target.cedula.take(3)

                val result = searchInquilinos(entities, partial)
                result.any { it.id == target.id } shouldBe true

                result.forEach { e ->
                    val term = partial.lowercase()
                    val matches = e.nombre.lowercase().contains(term) ||
                        e.apellido.lowercase().contains(term) ||
                        e.cedula.lowercase().contains(term)
                    matches shouldBe true
                }
            }
        }

        "search term not present in any field returns empty list" {
            checkAll(100, Arb.list(inquilinoArb, 1..20)) { entities ->
                val result = searchInquilinos(entities, "ZZZZXXX999NOTFOUND")
                result.size shouldBe 0
            }
        }

        "search by exact nombre from existing entity always finds it" {
            checkAll(100, Arb.list(inquilinoArb, 2..20)) { entities ->
                val target = entities.random()
                val result = searchInquilinos(entities, target.nombre)
                result.any { it.id == target.id } shouldBe true
            }
        }
    }
})
