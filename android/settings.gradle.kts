pluginManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "PropManager"

include(":app")

// Core modules
include(":core:model")
include(":core:common")
include(":core:database")
include(":core:network")
include(":core:data")
include(":core:ui")

// Feature modules
include(":feature:auth")
include(":feature:dashboard")
include(":feature:propiedades")
include(":feature:inquilinos")
include(":feature:contratos")
include(":feature:pagos")
include(":feature:gastos")
include(":feature:mantenimiento")
include(":feature:reportes")
include(":feature:documentos")
include(":feature:notificaciones")
include(":feature:auditoria")
include(":feature:perfil")
include(":feature:configuracion")
include(":feature:importacion")
include(":feature:scanner")
