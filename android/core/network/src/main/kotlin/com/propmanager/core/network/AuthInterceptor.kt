package com.propmanager.core.network

import javax.inject.Inject
import javax.inject.Singleton
import okhttp3.Interceptor
import okhttp3.Response

@Singleton
class AuthInterceptor @Inject constructor(private val tokenProvider: TokenProvider) : Interceptor {
    override fun intercept(chain: Interceptor.Chain): Response {
        val token = tokenProvider.getToken()
        val request =
            if (token != null) {
                chain.request().newBuilder().addHeader("Authorization", "Bearer $token").build()
            } else {
                chain.request()
            }
        return chain.proceed(request)
    }
}
