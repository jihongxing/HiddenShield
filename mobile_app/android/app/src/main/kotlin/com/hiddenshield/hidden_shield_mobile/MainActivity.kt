package com.hiddenshield.hidden_shield_mobile

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.DocumentsContract
import androidx.documentfile.provider.DocumentFile
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import kotlin.concurrent.thread

class MainActivity : FlutterActivity() {
    private var pendingTreeResult: MethodChannel.Result? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            RIGHTS_EVIDENCE_SAF_CHANNEL,
        ).setMethodCallHandler(::handleSafCall)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode != PICK_RIGHTS_EVIDENCE_TREE_REQUEST) {
            super.onActivityResult(requestCode, resultCode, data)
            return
        }

        val result = pendingTreeResult
        pendingTreeResult = null
        if (result == null) {
            return
        }
        if (resultCode != Activity.RESULT_OK) {
            result.success(null)
            return
        }

        val treeUri = data?.data
        if (treeUri == null) {
            result.error("missing_tree_uri", "系统文件选择器未返回目录 URI。", null)
            return
        }

        val grantedFlags =
            (data.flags and Intent.FLAG_GRANT_READ_URI_PERMISSION) or
                Intent.FLAG_GRANT_READ_URI_PERMISSION
        try {
            contentResolver.takePersistableUriPermission(treeUri, grantedFlags)
            preferences().edit().putString(PREF_TREE_URI, treeUri.toString()).apply()
            result.success(treeDescriptor(treeUri, persisted = true))
        } catch (error: SecurityException) {
            result.error(
                "persist_permission_failed",
                "无法持久保存案件包目录读取授权。",
                error.message,
            )
        }
    }

    private fun handleSafCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "pickTree" -> pickTree(result)
            "getPersistedTree" -> getPersistedTree(result)
            "clearPersistedTree" -> clearPersistedTree(result)
            "readFile" -> runIo(result) {
                val treeUri = requiredTreeUri(call)
                val relativePath = requiredRelativePath(call)
                readFile(treeUri, relativePath)
            }
            "listDirectory" -> runIo(result) {
                val treeUri = requiredTreeUri(call)
                listDirectory(treeUri)
            }
            else -> result.notImplemented()
        }
    }

    private fun pickTree(result: MethodChannel.Result) {
        if (pendingTreeResult != null) {
            result.error("picker_active", "系统文件选择器已打开。", null)
            return
        }
        pendingTreeResult = result
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            addFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
            addFlags(Intent.FLAG_GRANT_PREFIX_URI_PERMISSION)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                putExtra(
                    DocumentsContract.EXTRA_INITIAL_URI,
                    Uri.parse(DOWNLOADS_ROOT_URI),
                )
            }
        }
        startActivityForResult(intent, PICK_RIGHTS_EVIDENCE_TREE_REQUEST)
    }

    private fun getPersistedTree(result: MethodChannel.Result) {
        val stored = preferences().getString(PREF_TREE_URI, null)
        if (stored == null) {
            result.success(null)
            return
        }
        val uri = Uri.parse(stored)
        val stillGranted = contentResolver.persistedUriPermissions.any {
            it.uri == uri && it.isReadPermission
        }
        if (!stillGranted) {
            preferences().edit().remove(PREF_TREE_URI).apply()
            result.success(null)
            return
        }
        result.success(treeDescriptor(uri, persisted = true))
    }

    private fun clearPersistedTree(result: MethodChannel.Result) {
        val stored = preferences().getString(PREF_TREE_URI, null)
        if (stored != null) {
            val uri = Uri.parse(stored)
            try {
                contentResolver.releasePersistableUriPermission(
                    uri,
                    Intent.FLAG_GRANT_READ_URI_PERMISSION,
                )
            } catch (_: SecurityException) {
            }
        }
        preferences().edit().remove(PREF_TREE_URI).apply()
        result.success(null)
    }

    private fun readFile(treeUri: Uri, relativePath: String): ByteArray {
        val components = validateRelativePath(relativePath)
        var current =
            DocumentFile.fromTreeUri(this, treeUri)
                ?: throw SafException(
                    EVIDENCE_PACK_DIRECTORY_MISSING,
                    "案件包目录已移动或删除。",
                )
        if (!current.exists() || !current.isDirectory) {
            throw SafException(
                EVIDENCE_PACK_DIRECTORY_MISSING,
                "案件包目录已移动或删除。",
            )
        }
        for ((index, component) in components.withIndex()) {
            current =
                current.findFile(component)
                    ?: throw SafException(
                        if (relativePath.startsWith("attachments/")) {
                            EVIDENCE_PACK_ATTACHMENT_MISSING
                        } else {
                            EVIDENCE_PACK_DIRECTORY_MISSING
                        },
                        "案件包文件不存在：$relativePath",
                    )
            if (index < components.lastIndex && !current.isDirectory) {
                throw SafException("invalid_path", "案件包路径包含非目录节点：$relativePath")
            }
        }
        if (!current.isFile || current.isVirtual) {
            throw SafException("unsafe_file", "案件包目标不是可读取的普通文件：$relativePath")
        }
        return contentResolver.openInputStream(current.uri)?.use { it.readBytes() }
            ?: throw SafException("read_failed", "无法读取案件包文件：$relativePath")
    }

    private fun listDirectory(treeUri: Uri): Map<String, Any> {
        val root =
            DocumentFile.fromTreeUri(this, treeUri)
                ?: throw SafException(
                    EVIDENCE_PACK_DIRECTORY_MISSING,
                    "案件包目录已移动或删除。",
                )
        if (!root.exists() || !root.isDirectory) {
            throw SafException(
                EVIDENCE_PACK_DIRECTORY_MISSING,
                "案件包目录已移动或删除。",
            )
        }
        val rootEntries = root.listFiles()
        val rootTreeSafe =
            root.isDirectory &&
                !root.isVirtual &&
                rootEntries.all { entry ->
                    val name = entry.name
                    name != null &&
                        isSafePathComponent(name) &&
                        (entry.isFile || entry.isDirectory)
                }
        val topLevelEntries =
            rootEntries.mapNotNull { it.name }.sorted()
        val caseFile = rootEntries.singleOrNull { it.name == "case.json" }
        val manifestFile =
            rootEntries.singleOrNull { it.name == "case-manifest.json" }
        val attachmentsRoot =
            rootEntries.singleOrNull { it.name == "attachments" }
        val attachmentPaths = mutableListOf<String>()
        val attachmentTreeSafe =
            attachmentsRoot != null &&
                attachmentsRoot.isDirectory &&
                collectAttachmentPaths(
                    attachmentsRoot,
                    "attachments",
                    attachmentPaths,
                )
        attachmentPaths.sort()
        return mapOf(
            "topLevelEntries" to topLevelEntries,
            "attachmentPaths" to attachmentPaths,
            "caseFileSafe" to
                (rootTreeSafe && caseFile?.isFile == true && !caseFile.isVirtual),
            "manifestFileSafe" to
                (rootTreeSafe &&
                    manifestFile?.isFile == true &&
                    !manifestFile.isVirtual),
            "attachmentTreeSafe" to (rootTreeSafe && attachmentTreeSafe),
        )
    }

    private fun collectAttachmentPaths(
        directory: DocumentFile,
        relativeDirectory: String,
        output: MutableList<String>,
    ): Boolean {
        for (entry in directory.listFiles()) {
            val name = entry.name ?: return false
            if (!isSafePathComponent(name)) {
                return false
            }
            val relativePath = "$relativeDirectory/$name"
            when {
                entry.isDirectory -> {
                    if (!collectAttachmentPaths(entry, relativePath, output)) {
                        return false
                    }
                }
                entry.isFile && !entry.isVirtual -> output.add(relativePath)
                else -> return false
            }
        }
        return true
    }

    private fun requiredTreeUri(call: MethodCall): Uri {
        val raw = call.argument<String>("treeUri")
        if (raw.isNullOrBlank()) {
            throw SafException("missing_tree_uri", "缺少案件包 tree URI。")
        }
        val uri = Uri.parse(raw)
        val authority = uri.authority
        if (
            authority.isNullOrBlank() ||
                packageManager.resolveContentProvider(
                    authority,
                    0,
                ) == null
        ) {
            throw SafException(
                EVIDENCE_PACK_PROVIDER_UNAVAILABLE,
                "案件包文件提供方当前不可用。",
            )
        }
        val granted = contentResolver.persistedUriPermissions.any {
            it.uri == uri && it.isReadPermission
        }
        if (!granted) {
            throw SafException(
                EVIDENCE_PACK_AUTHORIZATION_REVOKED,
                "案件包目录读取授权已失效。",
            )
        }
        return uri
    }

    private fun requiredRelativePath(call: MethodCall): String {
        return call.argument<String>("relativePath")
            ?: throw SafException("missing_relative_path", "缺少案件包相对路径。")
    }

    private fun validateRelativePath(relativePath: String): List<String> {
        if (relativePath.isBlank() || relativePath.startsWith("/") || relativePath.contains("\\")) {
            throw SafException("invalid_relative_path", "案件包路径必须是安全的正斜杠相对路径。")
        }
        val components = relativePath.split("/")
        if (components.any { !isSafePathComponent(it) }) {
            throw SafException("invalid_relative_path", "案件包路径包含不安全组件。")
        }
        return components
    }

    private fun isSafePathComponent(component: String): Boolean {
        return component.isNotBlank() && component != "." && component != ".."
    }

    private fun treeDescriptor(uri: Uri, persisted: Boolean): Map<String, Any> {
        val displayName =
            try {
                DocumentFile.fromTreeUri(this, uri)?.name
            } catch (_: Exception) {
                null
            }
                ?: DocumentsContract.getTreeDocumentId(uri).substringAfterLast(":")
        return mapOf(
            "treeUri" to uri.toString(),
            "displayName" to displayName,
            "persisted" to persisted,
        )
    }

    private fun preferences() =
        getSharedPreferences(PREFS_NAME, MODE_PRIVATE)

    private fun runIo(result: MethodChannel.Result, operation: () -> Any) {
        thread(name = "rights-evidence-saf") {
            try {
                val value = operation()
                runOnUiThread { result.success(value) }
            } catch (error: SafException) {
                runOnUiThread { result.error(error.code, error.message, null) }
            } catch (error: SecurityException) {
                runOnUiThread {
                    result.error(
                        EVIDENCE_PACK_AUTHORIZATION_REVOKED,
                        "案件包目录读取授权已失效。",
                        error.message,
                    )
                }
            } catch (error: Exception) {
                runOnUiThread {
                    result.error(
                        "saf_io_failed",
                        "案件包目录读取失败。",
                        error.message,
                    )
                }
            }
        }
    }

    private class SafException(
        val code: String,
        override val message: String,
    ) : RuntimeException(message)

    companion object {
        private const val RIGHTS_EVIDENCE_SAF_CHANNEL =
            "com.hiddenshield.hidden_shield_mobile/rights_evidence_saf"
        private const val PICK_RIGHTS_EVIDENCE_TREE_REQUEST = 7412
        private const val PREFS_NAME = "rights_evidence_saf"
        private const val PREF_TREE_URI = "persisted_tree_uri"
        private const val DOWNLOADS_ROOT_URI =
            "content://com.android.providers.downloads.documents/root/downloads"
        private const val EVIDENCE_PACK_AUTHORIZATION_REVOKED =
            "evidence_pack_authorization_revoked"
        private const val EVIDENCE_PACK_DIRECTORY_MISSING =
            "evidence_pack_directory_missing"
        private const val EVIDENCE_PACK_ATTACHMENT_MISSING =
            "evidence_pack_attachment_missing"
        private const val EVIDENCE_PACK_PROVIDER_UNAVAILABLE =
            "evidence_pack_provider_unavailable"
    }
}
