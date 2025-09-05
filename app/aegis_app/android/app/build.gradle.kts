plugins {
    id("com.android.application")
    id("kotlin-android")
    // Muss nach Android/Kotlin kommen:
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "com.example.aegis_app"

    // Von Flutter vorgegeben (passt):
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    // >>> Wichtig: JDK 17 für AGP 8.x
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    defaultConfig {
        applicationId = "com.example.aegis_app"

        // >>> Wichtig: minSdk 21 (mobile_scanner braucht >=21)
        // Wenn dein flutter.minSdkVersion schon 21 ist, kannst du ihn auch nehmen.
        minSdk = 21

        // Lass target/versions von Flutter setzen:
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    buildTypes {
        release {
            // Für Tests: Debug-Signing (später eigenes Keystore)
            signingConfig = signingConfigs.getByName("debug")
            isMinifyEnabled = false
            isShrinkResources = false
        }
        debug {
            // Defaults ok
        }
    }
}

flutter {
    source = "../.."
}
