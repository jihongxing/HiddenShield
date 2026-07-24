import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/local_preview_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/workspace/rewrite_preflight.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';

void main() {
  const testSeed = WatermarkPayloadSeed(
    creatorIdentity: '\u0001\u0002\u0003\u0004',
    deviceIdentity: '\u0009\u000A',
    mediaBytes: [1, 2, 3, 4],
    timestamp: 1000,
  );

  test('plain preview bytes are classified as first write', () async {
    final state = MobileAppState(vaultStore: MemoryVaultStore());
    await state.load();

    final result = await inspectMobileRewriteTarget(
      bridge: const PreviewWatermarkBridge(),
      appState: state,
      kind: WatermarkAssetKind.image,
      bytes: const [1, 2, 3, 4],
    );

    expect(result.hasWatermark, isFalse);
    expect(result.nextRevision, 1);
    expect(result.reasonCode, 'no_valid_watermark');
    expect(preflightSummaryLabel(result), '未检测到已有隐盾水印');
    expect(preflightActionLabel(result), '将按首次写入处理');
    expect(preflightEvidenceLines(result), contains('如果继续生成保护副本，会创建新的版权记录。'));
  });

  test('watermarked preview bytes are classified as rewrite target', () async {
    final state = MobileAppState(vaultStore: MemoryVaultStore());
    await state.load();
    const bridge = PreviewWatermarkBridge();
    final written = await bridge.write(
      const WatermarkWriteRequest(
        kind: WatermarkAssetKind.audio,
        bytes: [1, 2, 3, 4],
        seed: testSeed,
      ),
    );

    expect(written.watermarkUid, 'PREVIEW-01020304090A');
    expect(written.watermarkUid.startsWith('HS-'), isFalse);

    final result = await inspectMobileRewriteTarget(
      bridge: bridge,
      appState: state,
      kind: WatermarkAssetKind.audio,
      bytes: written.bytes,
    );

    expect(result.hasWatermark, isTrue);
    expect(result.parentWatermarkUid, written.watermarkUid);
    expect(result.detectedRevision, 1);
    expect(result.nextRevision, 2);
    expect(result.reasonCode, 'rewrite_detected');
    expect(result.shouldBlockInitialWrite(allowRewrite: false), isTrue);
    expect(result.shouldBlockInitialWrite(allowRewrite: true), isFalse);
    expect(preflightSummaryLabel(result), '已检测到已有版权记录');
    expect(preflightActionLabel(result), '继续写入将记录为第 2 次写入');
    expect(
      preflightEvidenceLines(result),
      contains('上一版编号：PREVIEW-01020304090A'),
    );
    expect(
      existingWatermarkRewriteBlockedMessage(result.watermarkUid),
      '检测到已有版权记录 PREVIEW-01020304090A。如需生成新版，请开启“作为新版写入”。',
    );
  });

  test('watermarked preview bytes are rejected before second write', () async {
    const bridge = PreviewWatermarkBridge();
    final written = await bridge.write(
      const WatermarkWriteRequest(
        kind: WatermarkAssetKind.image,
        bytes: [1, 2, 3, 4],
        seed: testSeed,
      ),
    );

    await expectLater(
      bridge.write(
        WatermarkWriteRequest(
          kind: WatermarkAssetKind.image,
          bytes: written.bytes,
          seed: written.seed,
        ),
      ),
      throwsA(
        isA<StateError>().having(
          (error) => error.toString(),
          'message',
          contains(
            'watermark already exists in source media: PREVIEW-01020304090A',
          ),
        ),
      ),
    );
    expect(
      mobileWatermarkWriteErrorMessage(
        'watermark already exists in source media: PREVIEW-01020304090A',
      ),
      '检测到已有版权记录 PREVIEW-01020304090A。如需生成新版，请开启“作为新版写入”。',
    );
  });

  test('local vault revision is used when parent uid already exists', () async {
    final store = MemoryVaultStore();
    const bridge = _FixedReadBridge();
    final state = MobileAppState(vaultStore: store);
    await state.load();
    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.image,
        bytes: [1],
        watermarkUid: 'uid-existing',
        revision: 1,
        sha256: 'hash',
        seed: testSeed,
        processTimeMs: 1234,
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'cover.png',
      allowRewrite: true,
      parentWatermarkUid: 'uid-parent',
      revision: 3,
      rewriteReason: 'authorized rewrite',
    );

    final result = await inspectMobileRewriteTarget(
      bridge: bridge,
      appState: state,
      kind: WatermarkAssetKind.image,
      bytes: const [1, 2, 3],
    );

    expect(result.detectedRevision, 3);
    expect(result.nextRevision, 4);
    expect(result.rewriteReason, 'authorized rewrite');
    expect(preflightActionLabel(result), '继续写入将记录为第 4 次写入');
  });
}

class _FixedReadBridge extends WatermarkBridge {
  const _FixedReadBridge();

  @override
  Future<BridgeStatus> status() async {
    return const BridgeStatus(
      label: 'test',
      detail: 'test',
      capabilities: BridgeCapabilities(
        supportedKinds: [WatermarkAssetKind.image],
        supportsDesktopSync: false,
        supportsLocalVideo: false,
      ),
    );
  }

  @override
  Future<WatermarkReadResult?> read(WatermarkReadRequest request) async {
    return const WatermarkReadResult(
      kind: WatermarkAssetKind.image,
      watermarkUid: 'uid-existing',
      revision: 1,
      timestamp: 1000,
      deviceIdHex: 'device',
      fileHashHex: 'hash',
    );
  }

  @override
  Future<WatermarkWriteResult> write(WatermarkWriteRequest request) {
    throw UnimplementedError();
  }
}
