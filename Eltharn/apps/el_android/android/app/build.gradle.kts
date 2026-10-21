import java.util.Properties
import java.io.File

plugins {
    id("com.android.application")
}

val rustWorkspace = rootProject.projectDir.parentFile.parentFile.parentFile
val rustManifest = rustWorkspace.resolve("apps/el_android/Cargo.toml")
val rustTargetDirectory = System.getenv("CARGO_TARGET_DIR")
    ?.trim()
    ?.takeIf(String::isNotEmpty)
    ?.let { configured ->
        val directory = File(configured)
        if (directory.isAbsolute) directory else rustWorkspace.resolve(configured)
    }
    ?: rustWorkspace.parentFile.parentFile.resolve(".eltharn-build")
val localProperties = Properties()
val localPropertiesFile = rootProject.file("local.properties")

if (localPropertiesFile.isFile) {
    localPropertiesFile.inputStream().use(localProperties::load)
}

fun configuredAndroidPath(propertyName: String, vararg environmentNames: String): String? {
    val fromProperties = localProperties
        .getProperty(propertyName)
        ?.trim()
        ?.takeIf(String::isNotEmpty)
    if (fromProperties != null) {
        return fromProperties
    }

    return environmentNames
        .asSequence()
        .mapNotNull { environmentName ->
            System.getenv(environmentName)?.trim()?.takeIf(String::isNotEmpty)
        }
        .firstOrNull()
}


val configuredSdkDir = configuredAndroidPath(
    "sdk.dir",
    "ANDROID_SDK_ROOT",
    "ANDROID_HOME",
)
val configuredNdkDir = configuredAndroidPath(
    "ndk.dir",
    "ANDROID_NDK_HOME",
    "ANDROID_NDK_ROOT",
    "NDK_HOME",
)


val localDebugSigning =
    localProperties.getProperty("eltharn_gate.localDebugSigning").toBoolean() ||
        providers.gradleProperty("eltharn_gate.localDebugSigning").orNull.toBoolean()
val shaderProbe =
    providers.gradleProperty("eltharn.shaderProbe").orNull.toBoolean()
val verificationPackage =
    providers.gradleProperty("eltharn.verificationPackage").orNull.toBoolean()
if (shaderProbe) {
    layout.buildDirectory.set(layout.projectDirectory.dir("build/shaderprobe"))
}

android {
    configuredNdkDir?.let { directory ->
        ndkPath = directory
        val metadata = Properties()
        File(directory, "source.properties").inputStream().use(metadata::load)
        ndkVersion = metadata.getProperty("Pkg.Revision")
            ?: error("The configured Android NDK does not declare Pkg.Revision")
    }
    namespace = "dev.eltharn.gate"
    compileSdk = 34

    defaultConfig {
        applicationId = "dev.eltharn.gate"
        minSdk = 26
        targetSdk = 34
        versionCode = 2
        versionName = "0.1.0-alpha.2"
        manifestPlaceholders["appLabel"] = if (shaderProbe) "Eltharn Gate M03" else "Eltharn Gate"
        if (shaderProbe) {
            applicationIdSuffix = ".shaderprobe"
        }
        if (verificationPackage) {
            applicationIdSuffix = ".verification"
            manifestPlaceholders["appLabel"] = "Eltharn Gate Verification"
        }

        ndk {
            abiFilters += listOf("arm64-v8a")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    packaging {
        jniLibs.useLegacyPackaging = true
    }

    buildTypes {
        getByName("release") {
            if (localDebugSigning) {
                signingConfig = signingConfigs.getByName("debug")
            }
        }
    }
}


fun registerRustBuildTask(name: String, outputDirectory: File) =
    tasks.register<Exec>(name) {
        group = "eltharn"
        description = "Builds the packaged Rust Eltharn Gate lifecycle shell with cargo-ndk."
        workingDir = rustWorkspace

        val command = mutableListOf(
            "cargo",
            "ndk",
            "-t",
            "arm64-v8a",
            "-P",
            "26",
            "-o",
            outputDirectory.absolutePath,
            "build",
            "--locked",
            "--release",
            "--manifest-path",
            rustManifest.absolutePath,
            "-p",
            "el_android",
            "--features",
            if (shaderProbe) "extensions,shaderpacks,shader-probe" else "extensions,shaderpacks",
        )
        commandLine(command)

        configuredSdkDir?.let { sdkDir ->
            environment("ANDROID_HOME", sdkDir)
            environment("ANDROID_SDK_ROOT", sdkDir)
        }
        configuredNdkDir?.let { ndkDir ->
            environment("ANDROID_NDK_HOME", ndkDir)
            environment("ANDROID_NDK_ROOT", ndkDir)
            environment("NDK_HOME", ndkDir)
        }

        doFirst {
            check(configuredSdkDir != null) {
                "Android SDK is not configured. Set sdk.dir in local.properties or ANDROID_SDK_ROOT."
            }
            check(configuredNdkDir != null) {
                "Android NDK is not configured. Set ndk.dir in local.properties or ANDROID_NDK_HOME."
            }
        }


        inputs.file(rustManifest)
        inputs.file(rustWorkspace.parentFile.resolve("Cargo.toml"))
        inputs.file(rustWorkspace.parentFile.resolve("Cargo.lock"))
        inputs.file(rustWorkspace.parentFile.resolve("rust-toolchain.toml"))
        inputs.file(rustWorkspace.parentFile.resolve(".cargo/config.toml"))
        inputs.dir(rustWorkspace.resolve("apps/el_android/src"))
        inputs.dir(rustWorkspace.resolve("crates/el_app"))
        inputs.dir(rustWorkspace.resolve("crates/el_launcher_core"))
        inputs.dir(rustWorkspace.resolve("crates/el_platform_api"))


        inputs.dir(rustWorkspace.resolve("crates"))
        inputs.dir(rustWorkspace.resolve("crates/el_protocol_1_21_11"))
        inputs.dir(rustWorkspace.resolve("vendor"))
        inputs.property("shaderProbe", shaderProbe)
        outputs.dir(outputDirectory)
    }

val rustDebugDir = layout.buildDirectory.dir("generated/jniLibs/debug/renderer").get().asFile
val rustReleaseDir = layout.buildDirectory.dir("generated/jniLibs/release/renderer").get().asFile
val buildRustDebug = registerRustBuildTask(
    "buildRustDebug",
    rustDebugDir,
)
val buildRustRelease = registerRustBuildTask(
    "buildRustRelease",
    rustReleaseDir,
)

val backend12111Manifest = rustWorkspace.resolve("crates/el_protocol_1_21_11/Cargo.toml")
val buildBackend12111 = tasks.register<Exec>("buildBackend12111") {
    group = "eltharn"
    description = "Builds the ARM64 1.21.11 protocol backend for packaging in the APK."
    workingDir = rustWorkspace

    val command = mutableListOf(
        "cargo",
        "ndk",
        "-t",
        "arm64-v8a",
        "-P",
        "26",
        "build",
        "--locked",
        "--manifest-path",
        backend12111Manifest.absolutePath,
        "--release",
        "--bin",
        "eltharn-protocol",
    )
    commandLine(command)

    configuredSdkDir?.let { sdkDir ->
        environment("ANDROID_HOME", sdkDir)
        environment("ANDROID_SDK_ROOT", sdkDir)
    }
    configuredNdkDir?.let { ndkDir ->
        environment("ANDROID_NDK_HOME", ndkDir)
        environment("ANDROID_NDK_ROOT", ndkDir)
        environment("NDK_HOME", ndkDir)
    }

    inputs.file(backend12111Manifest)
    inputs.file(rustWorkspace.parentFile.resolve("Cargo.toml"))
    inputs.file(rustWorkspace.parentFile.resolve("Cargo.lock"))
    inputs.file(rustWorkspace.parentFile.resolve("rust-toolchain.toml"))
    inputs.file(rustWorkspace.parentFile.resolve(".cargo/config.toml"))
    inputs.dir(rustWorkspace.resolve("crates"))
    inputs.dir(rustWorkspace.resolve("vendor"))
    outputs.file(
        rustTargetDirectory.resolve("aarch64-linux-android/release/eltharn-protocol")
    )
}


val backendDebugDir = layout.buildDirectory.dir("generated/jniLibs/debug/backend").get().asFile
val backendReleaseDir = layout.buildDirectory.dir("generated/jniLibs/release/backend").get().asFile
fun registerBackendSyncTask(name: String, outputDirectory: File) =
    tasks.register<Sync>(name) {
        dependsOn(buildBackend12111)
        from(rustTargetDirectory.resolve("aarch64-linux-android/release/eltharn-protocol")) {
            rename { "libeltharn-protocol.so" }
        }
        into(outputDirectory.resolve("arm64-v8a"))
    }
val syncBackendDebug = registerBackendSyncTask("syncBackendDebug", backendDebugDir)
val syncBackendRelease = registerBackendSyncTask("syncBackendRelease", backendReleaseDir)

android.sourceSets.getByName("main").jniLibs.setSrcDirs(emptyList<String>())
android.sourceSets.getByName("debug").jniLibs.setSrcDirs(listOf(rustDebugDir, backendDebugDir))
android.sourceSets.getByName("release").jniLibs.setSrcDirs(listOf(rustReleaseDir, backendReleaseDir))


tasks.configureEach {
    when (name) {
        "preDebugBuild" -> dependsOn(buildRustDebug, syncBackendDebug)
        "preReleaseBuild" -> dependsOn(buildRustRelease, syncBackendRelease)
    }
}

tasks.configureEach {
    if (name == "buildBackend12111") {
        mustRunAfter("buildRustDebug", "buildRustRelease")
    }
}

dependencies {

    implementation("androidx.games:games-activity:2.0.2")


    implementation("androidx.appcompat:appcompat:1.6.1")
}
