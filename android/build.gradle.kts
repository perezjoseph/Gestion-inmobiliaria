plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.kotlin.jvm) apply false
    alias(libs.plugins.kotlin.serialization) apply false
    alias(libs.plugins.hilt) apply false
    alias(libs.plugins.ksp) apply false
    alias(libs.plugins.androidx.room) apply false
    alias(libs.plugins.detekt)
    alias(libs.plugins.spotless)
    alias(libs.plugins.sonarqube)
}

detekt {
    buildUponDefaultConfig = true
    allRules = false
    config.setFrom(files("$rootDir/detekt.yml"))
    source.setFrom(
        files(
            fileTree("app/src/main/kotlin"),
            fileTree("core") { include("**/src/main/kotlin/**") },
            fileTree("feature") { include("**/src/main/kotlin/**") },
        )
    )
    parallel = true
}

tasks.withType<io.gitlab.arturbosch.detekt.Detekt>().configureEach {
    reports {
        xml.required.set(true)
        xml.outputLocation.set(file("$rootDir/build/reports/detekt/detekt.xml"))
        html.required.set(true)
        html.outputLocation.set(file("$rootDir/build/reports/detekt/detekt.html"))
        sarif.required.set(false)
    }
}

subprojects {
    plugins.withId("com.android.application") {
        configure<com.android.build.api.dsl.ApplicationExtension> {
            buildTypes { getByName("debug") { enableUnitTestCoverage = true } }
        }
    }
    plugins.withId("com.android.library") {
        configure<com.android.build.api.dsl.LibraryExtension> {
            buildTypes { getByName("debug") { enableUnitTestCoverage = true } }
            compileOptions { isCoreLibraryDesugaringEnabled = true }
        }
        dependencies { add("coreLibraryDesugaring", "com.android.tools:desugar_jdk_libs:2.1.5") }
    }
}

spotless {
    kotlin {
        target("**/*.kt")
        targetExclude("**/build/**")
        ktfmt().kotlinlangStyle()
    }
    kotlinGradle {
        target("**/*.kts")
        targetExclude("**/build/**")
        ktfmt().kotlinlangStyle()
    }
}

sonar {
    properties {
        property("sonar.projectKey", System.getenv("SONAR_PROJECT_KEY") ?: "Gestion-inmobiliaria")
        property("sonar.projectName", System.getenv("SONAR_PROJECT_NAME") ?: "Gestion-inmobiliaria")
        property("sonar.host.url", System.getenv("SONAR_HOST_URL") ?: "http://sonar.local")
        property("sonar.token", System.getenv("SONAR_TOKEN") ?: "")
        property("sonar.sourceEncoding", "UTF-8")
        property("sonar.sources", "app/src/main,core,feature")
        property("sonar.tests", "app/src/test,app/src/androidTest")
        property("sonar.android.lint.reportPaths", "app/build/reports/lint-results-debug.xml")
        property("sonar.kotlin.detekt.reportPaths", "build/reports/detekt/detekt.xml")
    }
}
