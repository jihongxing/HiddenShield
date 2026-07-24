import 'dart:async';
import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/app/theme.dart';
import 'package:hidden_shield_mobile/bridge/rust_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/public_rights/public_metadata_embedder.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';
import 'package:hidden_shield_mobile/sync/cloud_account_client.dart';

const _backendUrl = String.fromEnvironment(
  'HIDDENSHIELD_QA_BACKEND_URL',
  defaultValue: 'http://127.0.0.1:43188',
);

const _runId = String.fromEnvironment('HIDDENSHIELD_QA_RUN_ID');

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const _IosPublicRightsV3QaApp());
}

class _IosPublicRightsV3QaApp extends StatefulWidget {
  const _IosPublicRightsV3QaApp();

  @override
  State<_IosPublicRightsV3QaApp> createState() =>
      _IosPublicRightsV3QaAppState();
}

class _IosPublicRightsV3QaAppState extends State<_IosPublicRightsV3QaApp> {
  _IosPublicRightsV3QaResult? _result;
  Object? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_run());
  }

  Future<void> _run() async {
    try {
      await _waitForBackendHealth();
      final result = await _runQa();
      debugPrint(
        'HIDDENSHIELD_IOS_PUBLIC_RIGHTS_QA_RESULT ${jsonEncode(result.toJson())}',
      );
      if (!result.passed) {
        throw StateError('iOS public rights V3 QA assertions failed');
      }
      if (mounted) setState(() => _result = result);
    } catch (error) {
      debugPrint('HIDDENSHIELD_IOS_PUBLIC_RIGHTS_QA_ERROR $error');
      if (mounted) setState(() => _error = error);
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield iOS Public Rights V3 QA',
      theme: buildHiddenShieldTheme(),
      home: Scaffold(
        body: SafeArea(
          child: _error != null
              ? _ErrorView(error: _error!)
              : _result == null
              ? const _LoadingView()
              : _ResultView(result: _result!),
        ),
      ),
    );
  }
}

class _LoadingView extends StatelessWidget {
  const _LoadingView();

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          CircularProgressIndicator(),
          SizedBox(height: 16),
          Text('正在执行 iOS 公开权利 / V3 运行态 QA'),
        ],
      ),
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.error});

  final Object error;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.error_outline, color: Colors.redAccent, size: 32),
          const SizedBox(height: 12),
          Text(
            'iOS 公开权利 QA 失败',
            style: Theme.of(context).textTheme.headlineSmall,
          ),
          const SizedBox(height: 12),
          Text('$error'),
        ],
      ),
    );
  }
}

class _ResultView extends StatelessWidget {
  const _ResultView({required this.result});

  final _IosPublicRightsV3QaResult result;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Row(
          children: [
            const Icon(Icons.verified_user_outlined),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'HiddenShield iOS 公开权利 / V3 QA',
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  const SizedBox(height: 4),
                  Text('后端 $_backendUrl · ${result.runId}'),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(14),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                _Detail(label: '版权编号', value: result.watermarkUid),
                _Detail(
                  label: '公开查询',
                  value: result.publicRightsJsonPass ? 'PASS' : 'FAIL',
                ),
                _Detail(
                  label: '公开元数据',
                  value: result.publicMetadataJsonPass ? 'PASS' : 'FAIL',
                ),
                _Detail(
                  label: '图片嵌入副本',
                  value: result.embeddedImagePass ? 'PASS' : 'FAIL',
                ),
                _Detail(
                  label: 'V3 默认写读',
                  value:
                      'V${result.payloadProtocolVersion} / ${result.payloadBytesLength} bytes · ${result.v3DefaultWriteReadPass ? 'PASS' : 'FAIL'}',
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        Text(
          '证据来自 iOS 原生 Flutter 运行态、真实 Rust watermark bridge、真实 feedback-backend public rights / metadata API 和 Dart 图片公开元数据嵌入器。',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

class _Detail extends StatelessWidget {
  const _Detail({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 112,
            child: Text(label, style: Theme.of(context).textTheme.bodySmall),
          ),
          Expanded(child: Text(value, overflow: TextOverflow.visible)),
        ],
      ),
    );
  }
}

Future<void> _waitForBackendHealth() async {
  final client = http.Client();
  try {
    final deadline = DateTime.now().add(const Duration(seconds: 60));
    Object? lastError;
    while (DateTime.now().isBefore(deadline)) {
      try {
        final response = await client
            .get(Uri.parse('$_backendUrl/v1/health'))
            .timeout(const Duration(seconds: 3));
        if (response.statusCode == 200) return;
        lastError = 'HTTP ${response.statusCode}';
      } catch (error) {
        lastError = error;
      }
      await Future<void>.delayed(const Duration(seconds: 1));
    }
    throw StateError('backend health unavailable from iOS: $lastError');
  } finally {
    client.close();
  }
}

Future<_IosPublicRightsV3QaResult> _runQa() async {
  final runId = _runId.isEmpty
      ? DateTime.now().millisecondsSinceEpoch.toString()
      : _runId;
  await RustWatermarkBridge.init();
  final bridge = RustWatermarkBridge();
  final cloudClient = CloudAccountClient(baseUrl: _backendUrl);
  final appState = MobileAppState(
    vaultStore: MemoryVaultStore(),
    cloudAccountClient: cloudClient,
  );
  await appState.load();
  final password = 'ios-public-rights-$runId';
  await appState.completeOnboarding(
    accountLabel: 'ios-public-rights-$runId@hiddenshield.local',
    password: password,
    creatorLabel: 'iOS 公开权利 QA 创作者',
  );
  await _upgradeToCreator(appState, cloudClient, password: password);

  final sourceBytes = _makePpmImage();
  final originalHash = appState.sha256HexForBytes(sourceBytes);
  final reserved = await appState.reserveWatermarkIdForWrite(
    kind: WatermarkAssetKind.image,
    originalHash: originalHash,
    revision: 1,
  );
  final writeResult = await bridge.write(
    WatermarkWriteRequest(
      kind: WatermarkAssetKind.image,
      bytes: sourceBytes,
      seed: WatermarkPayloadSeed(
        creatorIdentity: appState.creatorLabel,
        deviceIdentity: 'ios-public-rights-v3-qa',
        mediaBytes: sourceBytes,
        timestamp: DateTime.now().microsecondsSinceEpoch,
      ),
      allowRewrite: true,
      registryDraft: reserved?.toDraft(),
    ),
  );
  final confirmed = await appState.confirmWatermarkIdForWrite(
    result: writeResult,
    originalHash: originalHash,
    reserved: reserved,
  );
  final record = appState.addWriteResult(
    result: writeResult,
    fileName: 'ios-public-rights-v3-image-$runId',
    allowRewrite: false,
    registryResult: confirmed,
    declaration: const WorkDeclaration(
      workSourceDeclaration: 'ai_assisted',
      trainingPermissionDeclaration: 'commercial_allowed',
      creationMethodDeclaration: 'text_to_image',
      humanEditLevelDeclaration: 'light',
      authenticityClaimDeclaration: 'synthetic',
      customRightsStatement: 'iOS public rights V3 runtime QA',
    ),
  );

  await appState.syncPendingQueue();
  final rights = await appState.fetchPublicRights(record.watermarkUid);
  final metadata = await appState.fetchPublicRightsMetadata(
    record.watermarkUid,
  );
  final embedded = embedPublicRightsMetadataInImage(
    sourceBytes: Uint8List.fromList(writeResult.bytes),
    metadata: metadata,
    format: PublicMetadataImageFormat.png,
  );
  final manifestHash = metadata['manifestHash']?.toString() ?? '';
  final byteCheck = checkEmbeddedPublicMetadataBytes(
    bytes: embedded.bytes,
    format: PublicMetadataImageFormat.png,
    watermarkUid: record.watermarkUid,
    manifestHash: manifestHash,
  );

  final publicRightsJsonPass =
      rights.scanStatus == 'registry_active' &&
      rights.trainingPermission.policy == 'commercial_training_allowed' &&
      rights.trainingPermission.legalConclusion == false &&
      rights.registry.anchorProtocol == 'v3_minimal_anchor' &&
      rights.rightsManifest != null;
  final publicMetadataJsonPass =
      metadata['watermarkUid'] == record.watermarkUid &&
      metadata['legalConclusion'] == false &&
      manifestHash.isNotEmpty &&
      metadata['signedManifestStore'] is Map;
  final v3DefaultWriteReadPass =
      record.payloadProtocolVersion == 3 &&
      record.payloadBytesLength == 39 &&
      record.payloadAuthStatus == 'verified' &&
      record.watermarkIdRegistryStatus == 'server_confirmed';

  return _IosPublicRightsV3QaResult(
    runId: runId,
    watermarkUid: record.watermarkUid,
    publicRightsJsonPass: publicRightsJsonPass,
    publicMetadataJsonPass: publicMetadataJsonPass,
    embeddedImagePass: embedded.legalConclusion == false && byteCheck.pass,
    v3DefaultWriteReadPass: v3DefaultWriteReadPass,
    payloadProtocolVersion: record.payloadProtocolVersion,
    payloadBytesLength: record.payloadBytesLength,
    scanStatus: rights.scanStatus,
    trainingPolicy: rights.trainingPermission.policy,
    anchorProtocol: rights.registry.anchorProtocol,
    manifestHash: manifestHash,
    byteCheck: byteCheck.toJson(),
  );
}

Future<void> _upgradeToCreator(
  MobileAppState appState,
  CloudAccountClient client, {
  required String password,
}) async {
  final profile = appState.syncProfile;
  if (profile.entitlementFeatures['cloud_sync'] == true) {
    appState.setSyncTransportMode(SyncTransportMode.cloud);
    return;
  }
  final payment = await client.createBillingPaymentSession(
    accessToken: profile.authToken!,
    accountId: profile.accountId!,
    workspaceId: profile.workspaceId!,
    planCode: 'creator',
    preferredProvider: 'fixture',
  );
  await client.reconcileBillingPaymentSession(
    accessToken: profile.authToken!,
    paymentSessionId: payment.paymentSessionId,
  );
  await appState.continueWithCloudAccount(
    identifier: profile.accountLabel!,
    password: password,
    localCreatorDisplayName: appState.creatorLabel,
  );
  appState.setSyncTransportMode(SyncTransportMode.cloud);
}

Uint8List _makePpmImage() {
  const width = 512;
  const height = 512;
  final header = ascii.encode('P6\n$width $height\n255\n');
  final pixels = BytesBuilder();
  for (var y = 0; y < height; y++) {
    for (var x = 0; x < width; x++) {
      pixels.addByte((x * 255 ~/ width).clamp(0, 255));
      pixels.addByte((y * 255 ~/ height).clamp(0, 255));
      pixels.addByte(((sin((x + y) / 23) + 1) * 110).round().clamp(0, 255));
    }
  }
  return Uint8List.fromList([...header, ...pixels.toBytes()]);
}

class _IosPublicRightsV3QaResult {
  const _IosPublicRightsV3QaResult({
    required this.runId,
    required this.watermarkUid,
    required this.publicRightsJsonPass,
    required this.publicMetadataJsonPass,
    required this.embeddedImagePass,
    required this.v3DefaultWriteReadPass,
    required this.payloadProtocolVersion,
    required this.payloadBytesLength,
    required this.scanStatus,
    required this.trainingPolicy,
    required this.anchorProtocol,
    required this.manifestHash,
    required this.byteCheck,
  });

  final String runId;
  final String watermarkUid;
  final bool publicRightsJsonPass;
  final bool publicMetadataJsonPass;
  final bool embeddedImagePass;
  final bool v3DefaultWriteReadPass;
  final int payloadProtocolVersion;
  final int payloadBytesLength;
  final String scanStatus;
  final String trainingPolicy;
  final String anchorProtocol;
  final String manifestHash;
  final Map<String, Object?> byteCheck;

  bool get passed =>
      publicRightsJsonPass &&
      publicMetadataJsonPass &&
      embeddedImagePass &&
      v3DefaultWriteReadPass;

  Map<String, Object?> toJson() => {
    'runId': runId,
    'watermarkUid': watermarkUid,
    'platform': 'ios',
    'publicRightsJsonPass': publicRightsJsonPass,
    'publicMetadataJsonPass': publicMetadataJsonPass,
    'embeddedImagePass': embeddedImagePass,
    'v3DefaultWriteReadPass': v3DefaultWriteReadPass,
    'payloadProtocolVersion': payloadProtocolVersion,
    'payloadBytesLength': payloadBytesLength,
    'scanStatus': scanStatus,
    'trainingPolicy': trainingPolicy,
    'anchorProtocol': anchorProtocol,
    'manifestHash': manifestHash,
    'byteCheck': byteCheck,
    'legalConclusion': false,
    'passed': passed,
  };
}
