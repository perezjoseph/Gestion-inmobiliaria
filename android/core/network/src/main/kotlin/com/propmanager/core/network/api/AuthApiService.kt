package com.propmanager.core.network.api

import com.propmanager.core.model.dto.LoginRequest
import com.propmanager.core.model.dto.LoginResponse
import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.POST

interface AuthApiService {
    @POST("api/auth/login") suspend fun login(@Body request: LoginRequest): Response<LoginResponse>
}
