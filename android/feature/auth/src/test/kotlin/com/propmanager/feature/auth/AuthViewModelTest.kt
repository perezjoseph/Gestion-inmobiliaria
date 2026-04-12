package com.propmanager.feature.auth

import com.propmanager.core.model.UserProfile
import com.propmanager.core.model.dto.LoginRequest
import com.propmanager.core.model.dto.LoginResponse
import com.propmanager.core.model.dto.UserDto
import com.propmanager.core.network.TokenProvider
import com.propmanager.core.network.api.AuthApiService
import io.kotest.core.spec.style.FreeSpec
import io.kotest.matchers.nulls.shouldBeNull
import io.kotest.matchers.nulls.shouldNotBeNull
import io.kotest.matchers.shouldBe
import io.kotest.matchers.types.shouldBeInstanceOf
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.ResponseBody.Companion.toResponseBody
import retrofit2.Response

/**
 * Unit tests for AuthViewModel.
 *
 * Validates: Requirements 1.1, 1.2, 1.3, 1.4
 */
@OptIn(ExperimentalCoroutinesApi::class)
class AuthViewModelTest :
    FreeSpec({

        val testDispatcher = StandardTestDispatcher()

        beforeEach {
            Dispatchers.setMain(testDispatcher)
        }

        afterEach {
            Dispatchers.resetMain()
        }

        "successful login stores token and profile" {
            val fakeToken = FakeTokenProvider()
            val fakeApi =
                FakeAuthApiService(
                    loginResult =
                        Response.success(
                            LoginResponse(
                                token = "jwt-abc-123",
                                user =
                                    UserDto(
                                        id = "user-1",
                                        nombre = "Carlos Pérez",
                                        email = "carlos@example.com",
                                        rol = "gerente",
                                        activo = true,
                                        createdAt = "2025-01-01T00:00:00Z",
                                    ),
                            ),
                        ),
                )

            runTest(testDispatcher) {
                val vm = AuthViewModel(fakeApi, fakeToken)
                advanceUntilIdle()

                vm.onEmailChange("carlos@example.com")
                vm.onPasswordChange("secret123")
                vm.login()
                advanceUntilIdle()

                fakeToken.getToken() shouldBe "jwt-abc-123"
                val profile = fakeToken.getUserProfile()
                profile.shouldNotBeNull()
                profile.id shouldBe "user-1"
                profile.nombre shouldBe "Carlos Pérez"
                profile.email shouldBe "carlos@example.com"
                profile.rol shouldBe "gerente"

                vm.authState.value.shouldBeInstanceOf<AuthState.Authenticated>()
                vm.formState.value.isLoading shouldBe false
                vm.formState.value.errorMessage
                    .shouldBeNull()
            }
        }

        "failed login shows error message" {
            val fakeToken = FakeTokenProvider()
            val errorJson = """{"error":"unauthorized","message":"Credenciales inválidas"}"""
            val fakeApi =
                FakeAuthApiService(
                    loginResult =
                        Response.error(
                            401,
                            errorJson.toResponseBody("application/json".toMediaType()),
                        ),
                )

            runTest(testDispatcher) {
                val vm = AuthViewModel(fakeApi, fakeToken)
                advanceUntilIdle()

                vm.onEmailChange("bad@example.com")
                vm.onPasswordChange("wrong")
                vm.login()
                advanceUntilIdle()

                fakeToken.getToken().shouldBeNull()
                vm.formState.value.errorMessage
                    .shouldNotBeNull()
                vm.formState.value.isLoading shouldBe false
                vm.authState.value.shouldBeInstanceOf<AuthState.Unauthenticated>()
            }
        }

        "401 clears session via onSessionExpired" {
            val fakeToken =
                FakeTokenProvider().apply {
                    saveToken("existing-token")
                    saveUserProfile(
                        UserProfile(id = "u1", nombre = "Test", email = "t@t.com", rol = "gerente"),
                    )
                }
            val fakeApi = FakeAuthApiService()

            runTest(testDispatcher) {
                val vm = AuthViewModel(fakeApi, fakeToken)
                advanceUntilIdle()

                vm.authState.value.shouldBeInstanceOf<AuthState.Authenticated>()

                vm.onSessionExpired()

                fakeToken.getToken().shouldBeNull()
                fakeToken.getUserProfile().shouldBeNull()
                vm.authState.value.shouldBeInstanceOf<AuthState.Unauthenticated>()
                vm.formState.value.errorMessage
                    .shouldNotBeNull()
            }
        }

        "logout clears token and profile" {
            val fakeToken =
                FakeTokenProvider().apply {
                    saveToken("existing-token")
                    saveUserProfile(
                        UserProfile(id = "u1", nombre = "Test", email = "t@t.com", rol = "gerente"),
                    )
                }
            val fakeApi = FakeAuthApiService()

            runTest(testDispatcher) {
                val vm = AuthViewModel(fakeApi, fakeToken)
                advanceUntilIdle()

                vm.authState.value.shouldBeInstanceOf<AuthState.Authenticated>()

                vm.logout()

                fakeToken.getToken().shouldBeNull()
                fakeToken.getUserProfile().shouldBeNull()
                vm.authState.value.shouldBeInstanceOf<AuthState.Unauthenticated>()
            }
        }

        "login with blank fields shows validation errors without calling API" {
            val fakeApi = FakeAuthApiService()
            val fakeToken = FakeTokenProvider()

            runTest(testDispatcher) {
                val vm = AuthViewModel(fakeApi, fakeToken)
                advanceUntilIdle()

                vm.login()
                advanceUntilIdle()

                vm.formState.value.emailError
                    .shouldNotBeNull()
                vm.formState.value.passwordError
                    .shouldNotBeNull()
                fakeApi.loginCallCount shouldBe 0
            }
        }

        "network error shows offline message" {
            val fakeToken = FakeTokenProvider()
            val fakeApi = FakeAuthApiService(throwOnLogin = java.io.IOException("No network"))

            runTest(testDispatcher) {
                val vm = AuthViewModel(fakeApi, fakeToken)
                advanceUntilIdle()

                vm.onEmailChange("user@example.com")
                vm.onPasswordChange("pass")
                vm.login()
                advanceUntilIdle()

                vm.formState.value.errorMessage shouldBe "Sin conexión a internet"
                vm.formState.value.isLoading shouldBe false
            }
        }
    })

private class FakeTokenProvider : TokenProvider {
    private var token: String? = null
    private var profile: UserProfile? = null

    override fun getToken(): String? = token

    override fun saveToken(token: String) {
        this.token = token
    }

    override fun clearToken() {
        token = null
    }

    override fun saveUserProfile(user: UserProfile) {
        profile = user
    }

    override fun getUserProfile(): UserProfile? = profile

    override fun clearAll() {
        token = null
        profile = null
    }
}

private class FakeAuthApiService(
    private val loginResult: Response<LoginResponse>? = null,
    private val throwOnLogin: Throwable? = null,
) : AuthApiService {
    var loginCallCount = 0
        private set

    override suspend fun login(request: LoginRequest): Response<LoginResponse> {
        loginCallCount++
        throwOnLogin?.let { throw it }
        return loginResult ?: Response.error(
            500,
            "{}".toResponseBody("application/json".toMediaType()),
        )
    }
}
