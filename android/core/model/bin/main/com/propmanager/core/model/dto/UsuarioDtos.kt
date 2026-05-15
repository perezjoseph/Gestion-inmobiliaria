package com.propmanager.core.model.dto

import kotlinx.serialization.Serializable

/**
 * Request body for changing a user's role.
 * Used with PUT /api/usuarios/{id}/rol
 */
@Serializable
data class ChangeRoleRequest(val rol: String)
