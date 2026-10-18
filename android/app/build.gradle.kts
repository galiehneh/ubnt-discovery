import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
}

val signingPropertiesFile = rootProject.file("signing/release-signing.properties")
val signingProperties = Properties().apply {
    if (signingPropertiesFile.isFile) {
        signingPropertiesFile.inputStream().use { load(it) }
    }
}

android {
    namespace = "com.galiehneh.ubntdiscovery"
    compileSdk = 35

    lint {
        // versionCode is YYYYMMDD by project convention
        disable += "HighAppVersionCode"
    }

    defaultConfig {
        applicationId = "com.galiehneh.ubntdiscovery"
        minSdk = 24
        targetSdk = 35
        versionCode = 20261018
        versionName = "2026.10.18"
        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a")
        }
    }

    signingConfigs {
        if (signingPropertiesFile.isFile) {
            create("release") {
                fun prop(name: String) = requireNotNull(signingProperties.getProperty(name)) {
                    "Missing '$name' in signing/release-signing.properties"
                }
                storeFile = rootProject.file("signing").resolve(prop("storeFile"))
                storePassword = prop("storePassword")
                keyAlias = prop("keyAlias")
                keyPassword = prop("keyPassword")
                enableV1Signing = true
                enableV2Signing = true
                enableV3Signing = true
            }
        }
    }

    buildFeatures {
        compose = true
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            if (signingPropertiesFile.isFile) {
                signingConfig = signingConfigs.getByName("release")
            }
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

// Builds the Rust JNI library; pass -PskipCargo to use prebuilt .so files.
val cargoNdkBuild by tasks.registering(Exec::class) {
    workingDir = rootProject.projectDir.parentFile
    val cargo = File(System.getProperty("user.home"), ".cargo/bin/cargo").let {
        if (it.isFile) it.absolutePath else "cargo"
    }
    commandLine(
        cargo, "ndk", "-t", "arm64-v8a", "-t", "armeabi-v7a", "--platform", "24",
        "-o", "android/app/src/main/jniLibs", "build", "--release", "-p", "ubnt-discovery-jni",
    )
    onlyIf { !project.hasProperty("skipCargo") }
}

tasks.configureEach {
    if (name.startsWith("merge") && name.endsWith("JniLibFolders")) {
        dependsOn(cargoNdkBuild)
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2025.05.01")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
    implementation("androidx.core:core-ktx:1.13.1")
}
