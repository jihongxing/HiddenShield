plugins {
    id("com.android.application")
}

android {
    namespace = "com.hiddenshield.qa.documentsprovider"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.hiddenshield.qa.documentsprovider"
        minSdk = 21
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"
    }

    sourceSets {
        getByName("main") {
            assets.srcDir(
                file(
                    "../../../docs/fixtures/rights-evidence-pack-r4/case-fixture-r4-0001",
                ),
            )
        }
    }
}
