package health.app

import skip.lib.*
import skip.model.*
import skip.foundation.*
import skip.ui.*

import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.Dispatchers
import android.Manifest
import android.app.Application
import android.graphics.Color as AndroidColor
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.SystemBarStyle
import androidx.activity.ComponentActivity
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalContext
import androidx.compose.material3.MaterialTheme
import androidx.core.app.ActivityCompat

internal val logger: SkipLogger = SkipLogger(subsystem = "health.app", category = "HealthApp")

private typealias AppRootView = HealthAppRootView
private typealias AppDelegate = HealthAppAppDelegate

/// AndroidAppMain is the `android.app.Application` entry point, and must match `application android:name` in the AndroidMainfest.xml file.
open class AndroidAppMain: Application {
    constructor() {
    }

    override fun onCreate() {
        super.onCreate()
        logger.info("starting app")
        NativeServices.context = applicationContext
        ProcessInfo.launch(applicationContext)
        check(AppDelegate.shared.onInit(temporaryDirectory = cacheDir.absolutePath)) {
            "Could not configure the private download directory"
        }
    }

    companion object {
    }
}

/// AndroidAppMain is initial `androidx.appcompat.app.AppCompatActivity`, and must match `activity android:name` in the AndroidMainfest.xml file.
open class MainActivity: AppCompatActivity {
    private var permissionResult: kotlinx.coroutines.CompletableDeferred<Boolean>? = null
    private val permissionLauncher = registerForActivityResult(androidx.activity.result.contract.ActivityResultContracts.RequestPermission()) {
        permissionResult?.complete(it); permissionResult = null
    }
    suspend fun requestSystemPermission(permission: String): Boolean = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.Main) {
        if (androidx.core.content.ContextCompat.checkSelfPermission(this@MainActivity, permission) == android.content.pm.PackageManager.PERMISSION_GRANTED) return@withContext true
        check(permissionResult == null) { "Another permission request is open" }
        val result = kotlinx.coroutines.CompletableDeferred<Boolean>(); permissionResult = result
        try { permissionLauncher.launch(permission); result.await() }
        finally { if (permissionResult === result) permissionResult = null }
    }
    private var documentResult: kotlinx.coroutines.CompletableDeferred<List<android.net.Uri>>? = null
    private val documentLauncher = registerForActivityResult(androidx.activity.result.contract.ActivityResultContracts.StartActivityForResult()) { result ->
        val selected = mutableListOf<android.net.Uri>()
        if (result.resultCode == android.app.Activity.RESULT_OK) {
            result.data?.clipData?.let { clips -> for (index in 0 until clips.itemCount) selected.add(clips.getItemAt(index).uri) }
            if (selected.isEmpty()) result.data?.data?.let { selected.add(it) }
        }
        documentResult?.complete(selected); documentResult = null
    }
    suspend fun chooseDocuments(kind: String): List<android.net.Uri> {
        check(documentResult == null) { "A document chooser is already open" }
        val result = kotlinx.coroutines.CompletableDeferred<List<android.net.Uri>>()
        documentResult = result
        try {
            val intent = if (kind == "camera") android.content.Intent(this, ReportCaptureActivity::class.java)
                else android.content.Intent(android.content.Intent.ACTION_OPEN_DOCUMENT).apply {
                    addCategory(android.content.Intent.CATEGORY_OPENABLE)
                    type = if (kind == "pdf") "application/pdf" else if (kind == "training") "*/*" else "image/*"
                    putExtra(android.content.Intent.EXTRA_ALLOW_MULTIPLE, kind == "images")
                }
            documentLauncher.launch(intent)
            return result.await()
        } finally { if (documentResult === result) documentResult = null }
    }
    private var healthPermissionResult: kotlinx.coroutines.CompletableDeferred<Set<String>>? = null
    private val healthPermissionLauncher = registerForActivityResult(
        androidx.health.connect.client.PermissionController.createRequestPermissionResultContract()
    ) { granted -> healthPermissionResult?.complete(granted); healthPermissionResult = null }

    suspend fun requestHealthPermissions(extra: Set<String>): Set<String> = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.Main) {
        check(healthPermissionResult == null) { "A Health Connect permission request is already open" }
        val result = kotlinx.coroutines.CompletableDeferred<Set<String>>()
        healthPermissionResult = result
        try {
            healthPermissionLauncher.launch(HealthConnectService.permissions + extra)
            result.await()
        } finally { if (healthPermissionResult === result) healthPermissionResult = null }
    }

    constructor() {
    }

    override fun onCreate(savedInstanceState: android.os.Bundle?) {
        super.onCreate(savedInstanceState)
        logger.info("starting activity")
        NativeServices.activity = java.lang.ref.WeakReference(this)
        UIApplication.launch(this)
        enableEdgeToEdge()

        setContent {
            val saveableStateHolder = rememberSaveableStateHolder()
            saveableStateHolder.SaveableStateProvider(true) {
                PresentationRootView(ComposeContext())
                SideEffect { saveableStateHolder.removeState(true) }
            }
        }

        AppDelegate.shared.onLaunch()
        receiveSharedPDF(intent)

        // Example of requesting permissions on startup.
        // These must match the permissions in the AndroidManifest.xml file.
        //let permissions = listOf(
        //    Manifest.permission.ACCESS_COARSE_LOCATION,
        //    Manifest.permission.ACCESS_FINE_LOCATION
        //    Manifest.permission.CAMERA,
        //    Manifest.permission.WRITE_EXTERNAL_STORAGE,
        //)
        //let requestTag = 1
        //ActivityCompat.requestPermissions(self, permissions.toTypedArray(), requestTag)
    }

    override fun onNewIntent(intent: android.content.Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        receiveSharedPDF(intent)
    }

    private fun receiveSharedPDF(intent: android.content.Intent) {
        val uri = when (intent.action) {
            android.content.Intent.ACTION_SEND -> androidx.core.content.IntentCompat.getParcelableExtra(intent, android.content.Intent.EXTRA_STREAM, android.net.Uri::class.java)
            android.content.Intent.ACTION_VIEW -> intent.data
            else -> null
        } ?: return
        lifecycleScope.launch {
            try {
                val file = withContext(Dispatchers.IO) { DocumentServices.copyDocument(uri, "application/pdf") }
                if (AppDelegate.shared.receiveSharedPDF(path = file.getString("path"))) {
                    withContext(Dispatchers.IO) { java.io.File(file.getString("path")).delete() }
                    intent.action = null
                }
            } catch (cancelled: kotlinx.coroutines.CancellationException) { throw cancelled }
            catch (error: Exception) { AppDelegate.shared.sharedPDFError(message = error.message ?: "Could not import the shared PDF") }
        }
    }

    override fun onStart() {
        logger.info("onStart")
        super.onStart()
    }

    override fun onResume() {
        super.onResume()
        NativeServices.active = true
        AppDelegate.shared.onResume()
    }

    override fun onPause() {
        super.onPause()
        NativeServices.active = false
        AppDelegate.shared.onPause()
    }

    override fun onStop() {
        super.onStop()
        AppDelegate.shared.onStop()
    }

    override fun onDestroy() {
        super.onDestroy()
        AppDelegate.shared.onDestroy()
    }

    override fun onLowMemory() {
        super.onLowMemory()
        AppDelegate.shared.onLowMemory()
    }

    override fun onRestart() {
        logger.info("onRestart")
        super.onRestart()
    }

    override fun onSaveInstanceState(outState: android.os.Bundle): Unit = super.onSaveInstanceState(outState)

    override fun onRestoreInstanceState(bundle: android.os.Bundle) {
        // Usually you restore your state in onCreate(). It is possible to restore it in onRestoreInstanceState() as well, but not very common. (onRestoreInstanceState() is called after onStart(), whereas onCreate() is called before onStart().
        logger.info("onRestoreInstanceState")
        super.onRestoreInstanceState(bundle)
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: kotlin.Array<String>, grantResults: IntArray) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        logger.info("onRequestPermissionsResult: ${requestCode}")
    }

    companion object {
    }
}

@Composable
internal fun SyncSystemBarsWithTheme() {
    val dark = MaterialTheme.colorScheme.background.luminance() < 0.5f

    val transparent = AndroidColor.TRANSPARENT
    val style = if (dark) {
        SystemBarStyle.dark(transparent)
    } else {
        SystemBarStyle.light(transparent, transparent)
    }

    val activity = androidx.activity.compose.LocalActivity.current as? ComponentActivity
    DisposableEffect(style) {
        activity?.enableEdgeToEdge(
            statusBarStyle = style,
            navigationBarStyle = style
        )
        onDispose { }
    }
}

@Composable
internal fun PresentationRootView(context: ComposeContext) {
    val colorScheme = if (isSystemInDarkTheme()) ColorScheme.dark else ColorScheme.light
    HealthDesignTheme {
        PresentationRoot(defaultColorScheme = colorScheme, context = context) { ctx ->
            SyncSystemBarsWithTheme()
            val contentContext = ctx.content()
            Box(modifier = ctx.modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                AppRootView().healthDesign().Compose(context = contentContext)
            }
        }
    }
}
