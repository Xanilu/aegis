plugins {
    id "com.android.application"
    id "kotlin-android"
    id "dev.flutter.flutter-gradle-plugin"
}

android {
    namespace "com.example.aegis_app"
    compileSdkVersion 34

    defaultConfig {
        applicationId "com.example.aegis_app"
        minSdkVersion 21
        targetSdkVersion 34
        versionCode 1
        versionName "1.0"
        multiDexEnabled true
    }

    buildTypes {
        release {
            // Im Zweifel während der Entwicklung: kein Minify
            minifyEnabled false
            shrinkResources false
            // Wenn du signieren willst, füge signingConfig hier ein.
        }
        debug {
            // Debug-Defaults
        }
    }

    compileOptions {
        sourceCompatibility JavaVersion.VERSION_17
        targetCompatibility JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    // Für neuere Gradle-Plugins nicht nötig, aber falls du Probleme hast:
    // packagingOptions { jniLibs { useLegacyPackaging = true } }
}

flutter {
    source '../..'
}

dependencies {
    implementation "org.jetbrains.kotlin:kotlin-stdlib:1.9.24"
    // Weitere Abhängigkeiten zieht Flutter automatisch über pub.
}
