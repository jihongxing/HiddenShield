import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/local_preview_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/workspace/rewrite_preflight.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';

void main() {
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
  });

  test('watermarked preview bytes are classified as rewrite target', () async {
    final state = MobileAppState(vaultStore: MemoryVaultStore());
    await state.load();
    const bridge = PreviewWatermarkBridge();
    final written = await bridge.write(
      const WatermarkWriteRequest(
        kind: WatermarkAssetKind.audio,
        bytes: [1, 2, 3, 4],
        seed: WatermarkPayloadSeed(
          userSeed: [1, 2, 3, 4, 5, 6, 7, 8],
          timestamp: 1000,
          deviceId: [9, 10, 11, 12],
          fileHash: [13, 14],
        ),
      ),
    );

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
