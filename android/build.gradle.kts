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
            buildTypes {
                getByName("debug") {
                    enableUnitTestCoverage = true
                }
            }
        }
    }
    plugins.withId("com.android.library") {
        configure<com.android.build.api.dsl.LibraryExtension> {
            buildTypes {
                getByName("debug") {
                    enableUnitTestCoverage = true
                }
            }
        }
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
