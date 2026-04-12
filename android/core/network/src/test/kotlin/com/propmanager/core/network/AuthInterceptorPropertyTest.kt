package com.propmanager.core.network

import com.propmanager.core.model.UserProfile
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.nulls.shouldBeNull
import io.kotest.matchers.shouldBe
import io.kotest.property.Arb
import io.kotest.property.arbitrary.filter
import io.kotest.property.arbitrary.string
import io.kotest.property.checkAll
import okhttp3.Call
import okhttp3.Connection
import okhttp3.Interceptor
import okhttp3.Protocol
import okhttp3.Request
import okhttp3.Response
import java.util.concurrent.TimeUnit

/**
 * **Validates: Requirements 1.7**
 *
 * Property 2: Auth interceptor attaches Bearer token
 *
 * For any OkHttp request and any non-null stored JWT token string, passing the request
 * through AuthInterceptor.intercept() produces a request whose Authorization header
 * equals "Bearer {token}". When token is null, no Authorization header is added.
 */
class AuthInterceptorPropertyTest : FreeSpec({

    "Property 2: Auth interceptor attaches Bearer token" - {

        "non-null token attaches Bearer header to any request" {
            checkAll(100, Arb.string(1..200)) { token ->
                val provider = InMemoryTokenProvider(token)
                val interceptor = AuthInterceptor(provider)
                val originalRequest = Request.Builder()
                    .url("https://api.example.com/test")
                    .build()
                val chain = CapturingChain(originalRequest)

                interceptor.intercept(chain)

                val captured = chain.capturedRequest!!
                captured.header("Authorization") shouldBe "Bearer $token"
            }
        }

        "null token does not attach Authorization header" {
            checkAll(100, Arb.string(1..50)) { path ->
                val provider = InMemoryTokenProvider(null)
                val interceptor = AuthInterceptor(provider)
                val originalRequest = Request.Builder()
                    .url("https://api.example.com/$path")
                    .build()
                val chain = CapturingChain(originalRequest)

                interceptor.intercept(chain)

                val captured = chain.capturedRequest!!
                captured.header("Authorization").shouldBeNull()
            }
        }

        "original request headers are preserved when token is added" {
            checkAll(100, Arb.string(1..200), Arb.string(1..50).filter { it.isNotBlank() }) { token, customValue ->
                val provider = InMemoryTokenProvider(token)
                val interceptor = AuthInterceptor(provider)
                val originalRequest = Request.Builder()
                    .url("https://api.example.com/test")
                    .addHeader("X-Custom", customValue)
                    .build()
                val chain = CapturingChain(originalRequest)

                interceptor.intercept(chain)

                val captured = chain.capturedRequest!!
                captured.header("Authorization") shouldBe "Bearer $token"
                captured.header("X-Custom") shouldBe customValue
            }
        }
    }
})

private class InMemoryTokenProvider(private var token: String?) : TokenProvider {
    override fun getToken(): String? = token
    override fun saveToken(token: String) { this.token = token }
    override fun clearToken() { token = null }
    override fun saveUserProfile(user: UserProfile) {}
    override fun getUserProfile(): UserProfile? = null
    override fun clearAll() { token = null }
}

private class CapturingChain(private val originalRequest: Request) : Interceptor.Chain {
    var capturedRequest: Request? = null

    override fun request(): Request = originalRequest

    override fun proceed(request: Request): Response {
        capturedRequest = request
        return Response.Builder()
            .request(request)
            .protocol(Protocol.HTTP_1_1)
            .code(200)
            .message("OK")
            .build()
    }

    override fun connection(): Connection? = null
    override fun call(): Call = throw UnsupportedOperationException()
    override fun connectTimeoutMillis(): Int = 10_000
    override fun withConnectTimeout(timeout: Int, unit: TimeUnit): Interceptor.Chain = this
    override fun readTimeoutMillis(): Int = 10_000
    override fun withReadTimeout(timeout: Int, unit: TimeUnit): Interceptor.Chain = this
    override fun writeTimeoutMillis(): Int = 10_000
    override fun withWriteTimeout(timeout: Int, unit: TimeUnit): Interceptor.Chain = this
}
