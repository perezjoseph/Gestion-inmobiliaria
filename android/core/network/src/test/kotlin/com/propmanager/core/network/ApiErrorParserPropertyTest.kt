package com.propmanager.core.network

import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.property.Arb
import io.kotest.property.arbitrary.filter
import io.kotest.property.arbitrary.string
import io.kotest.property.checkAll
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.ResponseBody.Companion.toResponseBody
import retrofit2.Response

/**
 * **Validates: Requirements 19.1**
 *
 * Property 16: API error message extraction
 *
 * For any JSON string conforming to the backend error format {"error": "<type>", "message":
 * "<msg>"}, the error parser extracts the message field value exactly as provided, regardless of
 * the error type or message content. For malformed/non-JSON input, the parser returns a fallback
 * message without crashing.
 */
class ApiErrorParserPropertyTest :
    FreeSpec({
        "Property 16: API error message extraction" -
            {
                "valid JSON error body extracts message exactly" {
                    checkAll(
                        100,
                        Arb.string(1..100).filter { !it.contains('"') && !it.contains('\\') },
                        Arb.string(1..200).filter { !it.contains('"') && !it.contains('\\') },
                    ) { errorType, message ->
                        val jsonBody = """{"error": "$errorType", "message": "$message"}"""
                        val errorResponse =
                            Response.error<Unit>(
                                400,
                                jsonBody.toResponseBody("application/json".toMediaType()),
                            )

                        val extracted = ApiErrorParser.extractMessage(errorResponse)

                        extracted shouldBe message
                    }
                }

                "malformed JSON returns fallback message without crashing" {
                    checkAll(
                        100,
                        Arb.string(1..200).filter { input ->
                            runCatching {
                                    kotlinx.serialization.json.Json.decodeFromString<ApiErrorBody>(
                                        input
                                    )
                                }
                                .isFailure
                        },
                    ) { malformedInput ->
                        val errorResponse =
                            Response.error<Unit>(
                                400,
                                malformedInput.toResponseBody("application/json".toMediaType()),
                            )

                        val extracted = ApiErrorParser.extractMessage(errorResponse)

                        extracted shouldNotBe null
                        extracted.isNotBlank() shouldBe true
                    }
                }
            }
    })
