package com.propmanager.core.network

import com.propmanager.core.model.dto.LoginResponse
import com.propmanager.core.model.dto.UserDto
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.shouldBe
import io.kotest.property.Arb
import io.kotest.property.arbitrary.arbitrary
import io.kotest.property.arbitrary.boolean
import io.kotest.property.arbitrary.string
import io.kotest.property.checkAll
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * **Validates: Requirements 1.2**
 *
 * Property 1: Login response extraction round-trip
 *
 * For any valid LoginResponse DTO containing a user profile with arbitrary id, nombre,
 * email, and rol values, serializing to JSON and deserializing back yields the same object.
 */
class LoginResponseRoundTripPropertyTest :
    FreeSpec({

        val json = Json { ignoreUnknownKeys = true }

        val userDtoArb: Arb<UserDto> =
            arbitrary(
                edgecases =
                    listOf(
                        UserDto(
                            id = "00000000-0000-0000-0000-000000000000",
                            nombre = "Admin",
                            email = "admin@test.com",
                            rol = "gerente",
                            activo = true,
                            createdAt = "2024-01-01T00:00:00Z",
                        ),
                    ),
            ) {
                UserDto(
                    id = Arb.string(1..50).bind(),
                    nombre = Arb.string(1..100).bind(),
                    email = Arb.string(1..100).bind(),
                    rol = Arb.string(1..30).bind(),
                    activo = Arb.boolean().bind(),
                    createdAt = Arb.string(1..30).bind(),
                )
            }

        val loginResponseArb: Arb<LoginResponse> =
            arbitrary {
                LoginResponse(
                    token = Arb.string(1..200).bind(),
                    user = userDtoArb.bind(),
                )
            }

        "Property 1: Login response extraction round-trip" -
            {

                "serializing and deserializing LoginResponse preserves all fields" {
                    checkAll(100, loginResponseArb) { response ->
                        val serialized = json.encodeToString(response)
                        val deserialized = json.decodeFromString<LoginResponse>(serialized)
                        deserialized shouldBe response
                    }
                }

                "serialized JSON contains the createdAt field with SerialName mapping" {
                    checkAll(100, loginResponseArb) { response ->
                        val serialized = json.encodeToString(response)
                        val deserialized = json.decodeFromString<LoginResponse>(serialized)
                        deserialized.user.createdAt shouldBe response.user.createdAt
                    }
                }
            }
    })
