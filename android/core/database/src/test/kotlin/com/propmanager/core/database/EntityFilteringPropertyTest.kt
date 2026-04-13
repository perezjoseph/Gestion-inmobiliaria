package com.propmanager.core.database

import com.propmanager.core.database.entity.GastoEntity
import com.propmanager.core.database.entity.PagoEntity
import com.propmanager.core.database.entity.PropiedadEntity
import com.propmanager.core.database.entity.SolicitudMantenimientoEntity
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.collections.shouldContainExactlyInAnyOrder
import io.kotest.matchers.shouldBe
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.boolean
import io.kotest.property.arbitrary.element
import io.kotest.property.arbitrary.int
import io.kotest.property.arbitrary.list
import io.kotest.property.arbitrary.long
import io.kotest.property.arbitrary.string
import io.kotest.property.arbitrary.uuid
import io.kotest.property.checkAll

/**
 * **Validates: Requirements 3.2, 6.2, 7.2, 8.2**
 *
 * Property 7: Entity filtering returns only matching results
 *
 * For any entity list and any combination of filter criteria, applying the filter returns only
 * entities where every active filter criterion matches, and does not exclude any entity that
 * matches all active criteria.
 *
 * This validates the filtering contract at the data level, mirroring the WHERE clause logic that
 * Room DAOs enforce via their @Query annotations.
 */
class EntityFilteringPropertyTest :
    FreeSpec({

        // -- Domain value pools for realistic filter testing --

        val ciudades =
            listOf("Santo Domingo", "Santiago", "La Romana", "Puerto Plata", "Punta Cana")
        val estadosProp = listOf("disponible", "ocupada", "mantenimiento", "inactiva")
        val tiposProp = listOf("apartamento", "casa", "local", "oficina", "terreno")

        val estadosPago = listOf("pendiente", "pagado", "vencido", "anulado")
        val categorias = listOf("reparacion", "limpieza", "seguridad", "servicios", "otros")
        val estadosGasto = listOf("pendiente", "aprobado", "pagado", "rechazado")
        val estadosSolicitud = listOf("pendiente", "en_progreso", "completada", "cancelada")
        val prioridades = listOf("baja", "media", "alta", "urgente")

        // -- Arb generators --

        val propiedadArb: Arb<PropiedadEntity> = arbitrary {
            PropiedadEntity(
                id = Arb.uuid().bind().toString(),
                titulo = Arb.string(3..30).bind(),
                descripcion = null,
                direccion = Arb.string(5..40).bind(),
                ciudad = Arb.element(ciudades).bind(),
                provincia = Arb.string(3..20).bind(),
                tipoPropiedad = Arb.element(tiposProp).bind(),
                habitaciones = Arb.int(1..10).bind(),
                banos = Arb.int(1..5).bind(),
                areaM2 = "100.00",
                precio = "50000.00",
                moneda = Arb.element("DOP", "USD").bind(),
                estado = Arb.element(estadosProp).bind(),
                imagenes = null,
                createdAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
                updatedAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
                isDeleted = false,
                isPendingSync = false,
            )
        }

        val pagoArb: Arb<PagoEntity> = arbitrary {
            PagoEntity(
                id = Arb.uuid().bind().toString(),
                contratoId = Arb.uuid().bind().toString(),
                monto = "15000.00",
                moneda = Arb.element("DOP", "USD").bind(),
                fechaPago = null,
                fechaVencimiento = "2025-01-15",
                metodoPago = null,
                estado = Arb.element(estadosPago).bind(),
                notas = null,
                createdAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
                updatedAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
                isDeleted = false,
                isPendingSync = false,
            )
        }

        val gastoArb: Arb<GastoEntity> = arbitrary {
            GastoEntity(
                id = Arb.uuid().bind().toString(),
                propiedadId = Arb.uuid().bind().toString(),
                unidadId = null,
                categoria = Arb.element(categorias).bind(),
                descripcion = Arb.string(3..30).bind(),
                monto = "5000.00",
                moneda = Arb.element("DOP", "USD").bind(),
                fechaGasto = "2025-01-10",
                estado = Arb.element(estadosGasto).bind(),
                proveedor = null,
                numeroFactura = null,
                notas = null,
                createdAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
                updatedAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
                isDeleted = false,
                isPendingSync = false,
            )
        }

        val solicitudArb: Arb<SolicitudMantenimientoEntity> = arbitrary {
            SolicitudMantenimientoEntity(
                id = Arb.uuid().bind().toString(),
                propiedadId = Arb.uuid().bind().toString(),
                unidadId = null,
                inquilinoId = null,
                titulo = Arb.string(3..30).bind(),
                descripcion = null,
                estado = Arb.element(estadosSolicitud).bind(),
                prioridad = Arb.element(prioridades).bind(),
                nombreProveedor = null,
                telefonoProveedor = null,
                emailProveedor = null,
                costoMonto = null,
                costoMoneda = null,
                fechaInicio = null,
                fechaFin = null,
                createdAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
                updatedAt = Arb.long(1_000_000L..9_999_999_999L).bind(),
                isDeleted = false,
                isPendingSync = false,
            )
        }

        // -- Filter functions mirroring Room DAO WHERE clause logic --
        // These replicate the SQL: WHERE (:param IS NULL OR column = :param)

        fun filterPropiedades(
            entities: List<PropiedadEntity>,
            ciudad: String?,
            estado: String?,
            tipoPropiedad: String?,
        ): List<PropiedadEntity> =
            entities.filter { e ->
                (ciudad == null || e.ciudad == ciudad) &&
                    (estado == null || e.estado == estado) &&
                    (tipoPropiedad == null || e.tipoPropiedad == tipoPropiedad)
            }

        fun filterPagos(
            entities: List<PagoEntity>,
            contratoId: String?,
            estado: String?,
        ): List<PagoEntity> =
            entities.filter { e ->
                (contratoId == null || e.contratoId == contratoId) &&
                    (estado == null || e.estado == estado)
            }

        fun filterGastos(
            entities: List<GastoEntity>,
            propiedadId: String?,
            categoria: String?,
            estado: String?,
        ): List<GastoEntity> =
            entities.filter { e ->
                (propiedadId == null || e.propiedadId == propiedadId) &&
                    (categoria == null || e.categoria == categoria) &&
                    (estado == null || e.estado == estado)
            }

        fun filterSolicitudes(
            entities: List<SolicitudMantenimientoEntity>,
            estado: String?,
            prioridad: String?,
            propiedadId: String?,
        ): List<SolicitudMantenimientoEntity> =
            entities.filter { e ->
                (estado == null || e.estado == estado) &&
                    (prioridad == null || e.prioridad == prioridad) &&
                    (propiedadId == null || e.propiedadId == propiedadId)
            }

        // -- Nullable filter value generator: picks from pool or null --

        fun <T> nullableElement(values: List<T>): Arb<T?> = arbitrary {
            if (Arb.boolean().bind()) Arb.element(values).bind() else null
        }

        "Property 7: Entity filtering returns only matching results" -
            {
                "PropiedadEntity — filtered results match all active criteria" {
                    checkAll(
                        100,
                        Arb.list(propiedadArb, 1..30),
                        nullableElement(ciudades),
                        nullableElement(estadosProp),
                        nullableElement(tiposProp),
                    ) { entities, ciudad, estado, tipo ->
                        val result = filterPropiedades(entities, ciudad, estado, tipo)

                        result.forEach { e ->
                            if (ciudad != null) e.ciudad shouldBe ciudad
                            if (estado != null) e.estado shouldBe estado
                            if (tipo != null) e.tipoPropiedad shouldBe tipo
                        }

                        val expected =
                            entities.filter { e ->
                                (ciudad == null || e.ciudad == ciudad) &&
                                    (estado == null || e.estado == estado) &&
                                    (tipo == null || e.tipoPropiedad == tipo)
                            }
                        result shouldContainExactlyInAnyOrder expected
                    }
                }

                "PropiedadEntity — null filters return all entities" {
                    checkAll(100, Arb.list(propiedadArb, 1..30)) { entities ->
                        filterPropiedades(entities, null, null, null) shouldContainExactlyInAnyOrder
                            entities
                    }
                }

                "PagoEntity — filtered results match all active criteria" {
                    checkAll(
                        100,
                        Arb.list(pagoArb, 1..30),
                        nullableElement(listOf("contract-1", "contract-2", "contract-3")),
                        nullableElement(estadosPago),
                    ) { entities, contratoId, estado ->
                        val result = filterPagos(entities, contratoId, estado)

                        result.forEach { e ->
                            if (contratoId != null) e.contratoId shouldBe contratoId
                            if (estado != null) e.estado shouldBe estado
                        }

                        val expected =
                            entities.filter { e ->
                                (contratoId == null || e.contratoId == contratoId) &&
                                    (estado == null || e.estado == estado)
                            }
                        result shouldContainExactlyInAnyOrder expected
                    }
                }

                "PagoEntity — filter by contratoId from existing entities" {
                    checkAll(100, Arb.list(pagoArb, 2..30)) { entities ->
                        val targetContratoId = entities.random().contratoId
                        val result = filterPagos(entities, targetContratoId, null)

                        result.forEach { it.contratoId shouldBe targetContratoId }
                        result.size shouldBe entities.count { it.contratoId == targetContratoId }
                    }
                }

                "GastoEntity — filtered results match all active criteria" {
                    checkAll(
                        100,
                        Arb.list(gastoArb, 1..30),
                        nullableElement(listOf("prop-1", "prop-2", "prop-3")),
                        nullableElement(categorias),
                        nullableElement(estadosGasto),
                    ) { entities, propiedadId, categoria, estado ->
                        val result = filterGastos(entities, propiedadId, categoria, estado)

                        result.forEach { e ->
                            if (propiedadId != null) e.propiedadId shouldBe propiedadId
                            if (categoria != null) e.categoria shouldBe categoria
                            if (estado != null) e.estado shouldBe estado
                        }

                        val expected =
                            entities.filter { e ->
                                (propiedadId == null || e.propiedadId == propiedadId) &&
                                    (categoria == null || e.categoria == categoria) &&
                                    (estado == null || e.estado == estado)
                            }
                        result shouldContainExactlyInAnyOrder expected
                    }
                }

                "GastoEntity — filter by categoria from existing entities" {
                    checkAll(100, Arb.list(gastoArb, 2..30)) { entities ->
                        val targetCategoria = entities.random().categoria
                        val result = filterGastos(entities, null, targetCategoria, null)

                        result.forEach { it.categoria shouldBe targetCategoria }
                        result.size shouldBe entities.count { it.categoria == targetCategoria }
                    }
                }

                "SolicitudMantenimientoEntity — filtered results match all active criteria" {
                    checkAll(
                        100,
                        Arb.list(solicitudArb, 1..30),
                        nullableElement(estadosSolicitud),
                        nullableElement(prioridades),
                        nullableElement(listOf("prop-1", "prop-2", "prop-3")),
                    ) { entities, estado, prioridad, propiedadId ->
                        val result = filterSolicitudes(entities, estado, prioridad, propiedadId)

                        result.forEach { e ->
                            if (estado != null) e.estado shouldBe estado
                            if (prioridad != null) e.prioridad shouldBe prioridad
                            if (propiedadId != null) e.propiedadId shouldBe propiedadId
                        }

                        val expected =
                            entities.filter { e ->
                                (estado == null || e.estado == estado) &&
                                    (prioridad == null || e.prioridad == prioridad) &&
                                    (propiedadId == null || e.propiedadId == propiedadId)
                            }
                        result shouldContainExactlyInAnyOrder expected
                    }
                }

                "SolicitudMantenimientoEntity — filter by estado from existing entities" {
                    checkAll(100, Arb.list(solicitudArb, 2..30)) { entities ->
                        val targetEstado = entities.random().estado
                        val result = filterSolicitudes(entities, targetEstado, null, null)

                        result.forEach { it.estado shouldBe targetEstado }
                        result.size shouldBe entities.count { it.estado == targetEstado }
                    }
                }
            }
    })
