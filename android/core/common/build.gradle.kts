plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "com.propmanager.core.common"
    compileSdk = 36

    defaultConfig { minSdk = 24 }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    @Suppress("UnstableApiUsage") testOptions { unitTests.all { it.useJUnitPlatform() } }
}

dependencies {
    implementation(project(":core:model"))

    testImplementation(libs.bundles.unit.test)
    testImplementation(libs.bundles.kotest)
}
