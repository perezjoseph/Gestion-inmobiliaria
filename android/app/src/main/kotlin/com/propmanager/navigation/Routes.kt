package com.propmanager.navigation

object Routes {
    const val AUTH_GRAPH = "auth_graph"
    const val LOGIN = "login"

    const val MAIN_GRAPH = "main_graph"
    const val DASHBOARD = "dashboard"
    const val PROPIEDADES = "propiedades"
    const val PROPIEDAD_DETAIL = "propiedades/{id}"
    const val PROPIEDAD_FORM = "propiedades/form?id={id}"
    const val INQUILINOS = "inquilinos"
    const val INQUILINO_FORM = "inquilinos/form?id={id}"
    const val CONTRATOS = "contratos"
    const val CONTRATO_DETAIL = "contratos/{id}"
    const val CONTRATO_FORM = "contratos/form?id={id}"
    const val PAGOS = "pagos"
    const val PAGO_FORM = "pagos/form?id={id}"
    const val GASTOS = "gastos"
    const val GASTO_FORM = "gastos/form?id={id}"
    const val MANTENIMIENTO = "mantenimiento"
    const val SOLICITUD_DETAIL = "mantenimiento/{id}"
    const val SOLICITUD_FORM = "mantenimiento/form?id={id}"
    const val REPORTES = "reportes"
    const val DOCUMENTOS = "documentos/{entityType}/{entityId}"
    const val NOTIFICACIONES = "notificaciones"
    const val AUDITORIA = "auditoria"
    const val PERFIL = "perfil"
    const val CONFIGURACION = "configuracion"
    const val IMPORTACION = "importacion"
    const val SCANNER_CEDULA = "scanner/cedula"
    const val SCANNER_RECEIPT = "scanner/receipt"
    const val MAS = "mas"

    fun propiedadDetail(id: String) = "propiedades/$id"

    fun propiedadForm(id: String? = null) = "propiedades/form?id=${id ?: ""}"

    fun inquilinoForm(id: String? = null) = "inquilinos/form?id=${id ?: ""}"

    fun contratoDetail(id: String) = "contratos/$id"

    fun contratoForm(id: String? = null) = "contratos/form?id=${id ?: ""}"

    fun pagoForm(id: String? = null) = "pagos/form?id=${id ?: ""}"

    fun gastoForm(id: String? = null) = "gastos/form?id=${id ?: ""}"

    fun solicitudDetail(id: String) = "mantenimiento/$id"

    fun solicitudForm(id: String? = null) = "mantenimiento/form?id=${id ?: ""}"

    fun documentos(
        entityType: String,
        entityId: String,
    ) = "documentos/$entityType/$entityId"
}
