package com.propmanager.core.network

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.propmanager.core.model.UserProfile
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

interface TokenProvider {
    fun getToken(): String?
    fun saveToken(token: String)
    fun clearToken()
    fun saveUserProfile(user: UserProfile)
    fun getUserProfile(): UserProfile?
    fun clearAll()
}

@Singleton
class EncryptedTokenProvider @Inject constructor(
    @ApplicationContext context: Context
) : TokenProvider {

    private val prefs: SharedPreferences = EncryptedSharedPreferences.create(
        context,
        PREFS_NAME,
        MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build(),
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    override fun getToken(): String? = prefs.getString(KEY_TOKEN, null)

    override fun saveToken(token: String) {
        prefs.edit().putString(KEY_TOKEN, token).apply()
    }

    override fun clearToken() {
        prefs.edit().remove(KEY_TOKEN).apply()
    }

    override fun saveUserProfile(user: UserProfile) {
        prefs.edit()
            .putString(KEY_USER_ID, user.id)
            .putString(KEY_USER_NOMBRE, user.nombre)
            .putString(KEY_USER_EMAIL, user.email)
            .putString(KEY_USER_ROL, user.rol)
            .apply()
    }

    override fun getUserProfile(): UserProfile? {
        val id = prefs.getString(KEY_USER_ID, null) ?: return null
        val nombre = prefs.getString(KEY_USER_NOMBRE, null) ?: return null
        val email = prefs.getString(KEY_USER_EMAIL, null) ?: return null
        val rol = prefs.getString(KEY_USER_ROL, null) ?: return null
        return UserProfile(id = id, nombre = nombre, email = email, rol = rol)
    }

    override fun clearAll() {
        prefs.edit().clear().apply()
    }

    private companion object {
        const val PREFS_NAME = "propmanager_secure_prefs"
        const val KEY_TOKEN = "jwt_token"
        const val KEY_USER_ID = "user_id"
        const val KEY_USER_NOMBRE = "user_nombre"
        const val KEY_USER_EMAIL = "user_email"
        const val KEY_USER_ROL = "user_rol"
    }
}
