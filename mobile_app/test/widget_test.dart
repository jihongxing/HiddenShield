import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:hidden_shield_mobile/app/app.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';

void main() {
  testWidgets('renders the four main tabs', (WidgetTester tester) async {
    await tester.pumpWidget(const HiddenShieldApp());
    await tester.pumpAndSettle();

    expect(find.text('工作台'), findsWidgets);
    expect(find.text('取证'), findsWidgets);
    expect(find.text('版权库'), findsWidgets);
    expect(find.text('设置'), findsWidgets);
    expect(find.text('桥接层已接入'), findsOneWidget);
  });

  testWidgets('opens the image embed flow', (WidgetTester tester) async {
    await tester.pumpWidget(const HiddenShieldApp());
    await tester.pumpAndSettle();

    await tester.tap(find.text('图片嵌入'));
    await tester.pumpAndSettle();

    expect(find.text('选择图片'), findsOneWidget);
    expect(find.text('允许重写已有隐盾水印'), findsOneWidget);
    expect(find.text('写入盲水印'), findsOneWidget);
  });

  testWidgets('opens the audio embed flow', (WidgetTester tester) async {
    await tester.pumpWidget(const HiddenShieldApp());
    await tester.pumpAndSettle();

    await tester.tap(find.text('音频嵌入'));
    await tester.pumpAndSettle();

    expect(find.text('选择 WAV'), findsOneWidget);
    expect(find.text('允许重写已有隐盾水印'), findsOneWidget);
    expect(find.text('写入盲水印'), findsOneWidget);
  });

  testWidgets('renders the verify extraction flow', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(const HiddenShieldApp());
    await tester.pumpAndSettle();

    await tester.tap(find.text('取证').last);
    await tester.pumpAndSettle();

    expect(find.text('文件提取'), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(SegmentedButton<WatermarkAssetKind>),
        matching: find.text('图片'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byType(SegmentedButton<WatermarkAssetKind>),
        matching: find.text('WAV'),
      ),
      findsOneWidget,
    );
    expect(find.text('选择文件'), findsOneWidget);
    await tester.drag(find.byType(ListView).last, const Offset(0, -300));
    await tester.pumpAndSettle();
    expect(find.text('提取水印'), findsOneWidget);
  });

  testWidgets('renders mobile sync resolution summary in settings', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    await store.recordSyncResolution(
      MobileSyncResolution(
        id: 'resolution-1',
        resolvedAt: DateTime.fromMillisecondsSinceEpoch(1000),
        resolutionType: MobileSyncResolutionType.variantAccepted,
        reason: 'same watermark uid but different asset fingerprint',
        incomingRecordId: 'desktop:variant-1',
        watermarkUid: 'uid-variant',
        incomingRevision: 2,
        insertedRecordId: 'desktop:variant-1',
      ),
    );
    final state = MobileAppState(vaultStore: store);
    await state.load();

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();

    expect(find.text('自动解决审计'), findsOneWidget);
    expect(find.text('接收变体 · uid-variant · v2'), findsOneWidget);
    expect(find.text('累计'), findsOneWidget);
  });

  testWidgets('renders sync diagnostics in settings', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    await store.enqueueSyncItem(
      SyncQueueItem(
        id: 'queue-failed',
        recordId: 'record-failed',
        operation: SyncQueueOperation.upsertVaultRecord,
        payloadType: 'vault_record',
        payloadJson: '{}',
        status: SyncQueueItemStatus.failed,
        attempts: 1,
        createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
        lastError: 'network failed',
      ),
    );
    await store.saveSyncProfile(
      SyncProfile(
        mode: SyncTransportMode.lanDebug,
        lanDebugAddress: 'http://127.0.0.1:47219',
        lanDebugPairingCode: 'abcdef',
        status: SyncConnectionStatus.failed,
        updatedAt: DateTime.fromMillisecondsSinceEpoch(2000),
        lastError: 'pairing rejected',
        lastRemotePullCursor: '2026-06-16T12:00:00.000Z',
      ),
    );
    final state = MobileAppState(vaultStore: store);
    await state.load();

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();
    await tester.drag(find.byType(ListView).last, const Offset(0, -520));
    await tester.pumpAndSettle();

    expect(find.text('同步诊断'), findsOneWidget);
    expect(find.text('连接失败'), findsWidgets);
    expect(find.text('待同步 0 · 失败 1'), findsOneWidget);
    expect(find.textContaining('pairing rejected'), findsWidgets);
    expect(find.text('重试失败'), findsOneWidget);
  });

  testWidgets('renders account identity contract in settings', (
    WidgetTester tester,
  ) async {
    final state = MobileAppState(vaultStore: MemoryVaultStore());
    await state.load();
    state.updateCreatorLabel('Alice Creator');
    await state.continueWithAccountPlaceholder(accountLabel: 'alice@example.com');

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();

    expect(find.text('alice@example.com'), findsWidgets);
    expect(find.text('个人空间'), findsWidgets);
    expect(find.textContaining('当前移动设备'), findsWidgets);
    expect(find.text('云同步'), findsWidgets);
    expect(find.textContaining('creator_'), findsWidgets);
  });

  testWidgets('renders mobile pairing checklist in settings', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    await store.saveSyncProfile(
      SyncProfile(
        mode: SyncTransportMode.lanDebug,
        lanDebugAddress: 'http://192.168.1.8:47219',
        lanDebugPairingCode: '123456',
        status: SyncConnectionStatus.connected,
        updatedAt: DateTime.fromMillisecondsSinceEpoch(1000),
      ),
    );
    final state = MobileAppState(vaultStore: store);
    await state.load();
    state.setSyncTransportMode(SyncTransportMode.lanDebug);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(
      find.text('局域网调试同步').first,
      120,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byType(ExpansionTile).first);
    await tester.pumpAndSettle();

    expect(find.text('联调检查'), findsOneWidget);
    expect(find.text('http://192.168.1.8:47219'), findsWidgets);
    expect(find.text('已保存'), findsOneWidget);
    expect(find.text('局域网调试'), findsWidgets);
  });

  testWidgets('opens vault record details sheet', (WidgetTester tester) async {
    final store = MemoryVaultStore();
    await store.upsertRecord(
      VaultRecord(
        id: 'record-1',
        kind: WatermarkAssetKind.audio,
        title: 'song.wav',
        watermarkUid: 'uid-audio',
        revision: 3,
        sha256: 'abcdef1234567890',
        parentWatermarkUid: 'uid-parent',
        rewriteReason: 'authorized rewrite',
        extractedTimestamp: 123,
        extractedDeviceIdHex: 'device',
        extractedFileHashHex: 'hash',
        source: VaultRecordSource.verify,
        syncStatus: SyncStatus.synced,
        createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
      ),
    );
    final state = MobileAppState(vaultStore: store);
    await state.load();

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('版权库').last);
    await tester.pumpAndSettle();
    await tester.drag(find.byType(ListView).first, const Offset(0, -520));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('vault-record-record-1')));
    await tester.pumpAndSettle();

    expect(find.text('水印信息'), findsOneWidget);
    expect(find.text('文件指纹'), findsOneWidget);
    expect(find.text('第 3 次'), findsOneWidget);
    expect(find.text('uid-audio'), findsOneWidget);

    await tester.drag(find.byType(ListView).last, const Offset(0, -420));
    await tester.pumpAndSettle();

    expect(find.text('取证字段'), findsOneWidget);
    expect(find.text('本地记录'), findsOneWidget);
  });

  testWidgets('filters vault records by search and source', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    await store.upsertRecord(
      VaultRecord(
        id: 'record-write',
        kind: WatermarkAssetKind.image,
        title: 'cover.png',
        watermarkUid: 'uid-cover',
        revision: 1,
        sha256: 'hash-cover',
        source: VaultRecordSource.write,
        syncStatus: SyncStatus.pending,
        createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
      ),
    );
    await store.upsertRecord(
      VaultRecord(
        id: 'record-evidence',
        kind: WatermarkAssetKind.audio,
        title: 'suspect.wav',
        watermarkUid: 'uid-evidence',
        revision: 2,
        extractedDeviceIdHex: 'device-evidence',
        extractedFileHashHex: 'hash-evidence',
        source: VaultRecordSource.verify,
        syncStatus: SyncStatus.synced,
        createdAt: DateTime.fromMillisecondsSinceEpoch(2000),
      ),
    );
    final state = MobileAppState(vaultStore: store);
    await state.load();

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('版权库').last);
    await tester.pumpAndSettle();

    expect(find.text('cover.png'), findsOneWidget);
    expect(find.text('suspect.wav'), findsOneWidget);

    await tester.enterText(find.byType(TextField).last, 'uid-evidence');
    await tester.pumpAndSettle();

    expect(find.text('cover.png'), findsNothing);
    expect(find.text('suspect.wav'), findsOneWidget);
    expect(find.text('显示 1 / 2 条记录'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('vault-filter-取证')));
    await tester.pumpAndSettle();

    expect(find.text('suspect.wav'), findsOneWidget);
    expect(find.text('显示 1 / 2 条记录'), findsOneWidget);
  });
}
