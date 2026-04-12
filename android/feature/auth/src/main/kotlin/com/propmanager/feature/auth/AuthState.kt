package com.propmanager.feature.auth

import com.propmanager.core.model.UserProfile

sealed class AuthState {
    data object Loading : AuthState()
    data class Authenticated(val user: UserProfile) : AuthState()
    data object Unauthenticated : AuthState()
}
