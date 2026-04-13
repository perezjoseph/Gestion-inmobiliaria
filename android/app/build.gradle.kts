plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.hilt)
    alias(libs.plugins.ksp)
}

android {
    namespace = "com.propmanager"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.propmanager"
        minSdk = 24
        targetSdk = 36
        versionCode = 1
        versionName = "1.0.0"
    }

    compileOptions {
        isCoreLibraryDesugaringEnabled = true
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlinOptions { jvmTarget = "11" }

    buildFeatures { compose = true }
}

dependencies {
    // Feature modules
    implementation(project(":feature:auth"))
    implementation(project(":feature:dashboard"))
    implementation(project(":feature:propiedades"))
    implementation(project(":feature:inquilinos"))
    implementation(project(":feature:contratos"))
    implementation(project(":feature:pagos"))
    implementation(project(":feature:gastos"))
    implementation(project(":feature:mantenimiento"))
    implementation(project(":feature:reportes"))
    implementation(project(":feature:documentos"))
    implementation(project(":feature:notificaciones"))
    implementation(project(":feature:auditoria"))
    implementation(project(":feature:perfil"))
    implementation(project(":feature:configuracion"))
    implementation(project(":feature:importacion"))
    implementation(project(":feature:scanner"))

    // Core modules
    implementation(project(":core:ui"))
    implementation(project(":core:data"))
    implementation(project(":core:network"))
    implementation(project(":core:common"))
    implementation(project(":core:model"))

    // Compose
    val composeBom = platform(libs.androidx.compose.bom)
    implementation(composeBom)
    implementation(libs.bundles.compose)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.navigation.compose)

    // Hilt
    implementation(libs.hilt.android)
    ksp(libs.hilt.compiler)
    implementation(libs.hilt.navigation.compose)

    // Serialization (for error parsing)
    implementation(libs.kotlinx.serialization)

    // WorkManager
    implementation(libs.androidx.work.runtime.ktx)

    // Core library desugaring
    coreLibraryDesugaring(libs.androidx.core.desugaring)

    // Lifecycle
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)

    // Testing
    testImplementation(libs.bundles.unit.test)
    testImplementation(libs.bundles.kotest)
    androidTestImplementation(libs.bundles.android.test)
}
