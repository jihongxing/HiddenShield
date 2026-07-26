import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:hidden_shield_mobile/app/app.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/bridge/local_preview_watermark_bridge.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/features/verify/verify_page.dart';
import 'package:hidden_shield_mobile/features/vault/rights_evidence_pack_file_reader.dart';
import 'package:hidden_shield_mobile/features/vault/rights_evidence_pack_verifier.dart';
import 'package:hidden_shield_mobile/features/workspace/image_embed_page.dart';
import 'package:hidden_shield_mobile/licensing/offline_license_manager.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';

void main() {
  testWidgets('renders the five main tabs', (WidgetTester tester) async {
    final state = await _readyAppState();
    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    expect(find.text('工作台'), findsWidgets);
    expect(find.text('验证'), findsWidgets);
    expect(find.text('版权库'), findsWidgets);
    expect(find.text('批量'), findsWidgets);
    expect(find.text('设置'), findsWidgets);
    expect(find.text('桥接层已接入'), findsNothing);
  });

  testWidgets('opens the adaptive embed flow', (WidgetTester tester) async {
    final state = await _readyAppState();
    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.scrollUntilVisible(
      find.text('作品写入'),
      160,
      scrollable: _visibleScrollable(),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('作品写入').last);
    await tester.pumpAndSettle();

    expect(find.text('作品写入'), findsWidgets);
    expect(find.text('选择作品'), findsWidgets);
    expect(find.text('导入图片或音频作品'), findsOneWidget);
    expect(
      find.text('支持 JPG / PNG / WebP 和 WAV / MP3 / AAC / FLAC / OGG / M4A'),
      findsOneWidget,
    );
    expect(find.text('图片写入'), findsNothing);
    expect(find.text('音频写入'), findsNothing);
  });

  testWidgets('mobile write pages include declaration fields', (
    WidgetTester tester,
  ) async {
    final state = await _readyAppState();
    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    Navigator.of(tester.element(find.text('工作台').first)).push(
      MaterialPageRoute<void>(
        builder: (_) => ImageEmbedPage(
          bridge: const PreviewWatermarkBridge(),
          appState: state,
          onOpenVault: () {},
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.scrollUntilVisible(
      find.text('作品声明与授权策略'),
      180,
      scrollable: _visibleScrollable(),
    );
    await tester.pumpAndSettle();
    expect(find.text('作品声明与授权策略'), findsOneWidget);
    await tester.tap(find.text('记录创作者声明'));
    await tester.pumpAndSettle();
    expect(find.text('训练许可声明'), findsOneWidget);
    expect(find.text('禁止模型训练'), findsOneWidget);
    expect(find.textContaining('不检测 AI'), findsOneWidget);
  });

  testWidgets('renders the verify extraction flow', (
    WidgetTester tester,
  ) async {
    final state = await _readyAppState();
    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('验证').last);
    await tester.pumpAndSettle();

    expect(find.text('选择样本'), findsOneWidget);
    expect(find.byType(SegmentedButton<WatermarkAssetKind>), findsNothing);
    expect(find.text('选择需要验证的图片、音频或 L1 视频音轨样本'), findsOneWidget);
    expect(find.text('选择文件'), findsOneWidget);
    expect(find.text('检测范围'), findsOneWidget);
    await tester.drag(find.byType(ListView).last, const Offset(0, -300));
    await tester.pumpAndSettle();
    expect(find.text('开始检查'), findsOneWidget);
  });

  testWidgets('verifies an R4 rights evidence pack from the verify page', (
    WidgetTester tester,
  ) async {
    final state = await _readyAppState();
    final fixtureDir =
        '${Directory.current.path}${Platform.pathSeparator}'
        'test${Platform.pathSeparator}fixtures${Platform.pathSeparator}'
        'rights_evidence_pack_r4${Platform.pathSeparator}'
        'case-fixture-r4-0001';
    const fixtureFiles = [
      'case.json',
      'case-manifest.json',
      'attachments/original/ATT-01-original-work.txt',
      'attachments/working-copy/ATT-02-analysis-copy.txt',
      'attachments/capture/ATT-03-disputed-page-capture.txt',
      'attachments/external-receipt/ATT-04-platform-receipt.json',
    ];
    final fixtureBytes = {
      for (final relativePath in fixtureFiles)
        relativePath: File(
          '$fixtureDir${Platform.pathSeparator}'
          '${relativePath.replaceAll('/', Platform.pathSeparator)}',
        ).readAsBytesSync(),
    };
    final verifier = RightsEvidencePackVerifier(
      readBytes: (_, relativePath) async => fixtureBytes[relativePath]!,
      readDirectory: (_) async => const RightsEvidencePackDirectoryListing(
        topLevelEntries: ['attachments', 'case-manifest.json', 'case.json'],
        attachmentPaths: [
          'attachments/capture/ATT-03-disputed-page-capture.txt',
          'attachments/external-receipt/ATT-04-platform-receipt.json',
          'attachments/original/ATT-01-original-work.txt',
          'attachments/working-copy/ATT-02-analysis-copy.txt',
        ],
        caseFileSafe: true,
        manifestFileSafe: true,
        attachmentTreeSafe: true,
      ),
    );
    await tester.pumpWidget(
      MaterialApp(
        home: VerifyPage(
          bridge: const PreviewWatermarkBridge(),
          appState: state,
          pickRightsEvidencePackDirectory: () async => fixtureDir,
          rightsEvidencePackVerifier: verifier,
        ),
      ),
    );
    await tester.pumpAndSettle();

    final button = find.byKey(
      const ValueKey('verify-rights-evidence-pack-button'),
    );
    await tester.scrollUntilVisible(
      button,
      220,
      scrollable: _visibleScrollable(),
    );
    await tester.tap(button);
    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey('rights-evidence-status-directory')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('rights-evidence-status-attachments')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('rights-evidence-status-events')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('rights-evidence-status-attachment-chain')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('rights-evidence-status-signature')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('rights-evidence-status-trusted-time')),
      findsOneWidget,
    );
    expect(find.text('未签名'), findsOneWidget);
    expect(find.text('未加盖'), findsOneWidget);
    expect(
      find.text(
        '4b755ee5bc18384898174cb2046ea44e99983767d4c228760569bcf18ad64a33',
      ),
      findsNWidgets(2),
    );
    expect(
      find.byKey(const ValueKey('rights-evidence-pack-boundary')),
      findsOneWidget,
    );
  });

  testWidgets('renders mobile sync handling summary in settings', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    await store.recordSyncResolution(
      MobileSyncResolution(
        id: 'resolution-1',
        resolvedAt: DateTime.fromMillisecondsSinceEpoch(1000),
        resolutionType: MobileSyncResolutionType.pendingRegistryReconcile,
        reason:
            'same watermark uid but different asset fingerprint requires backend registry arbitration',
        incomingRecordId: 'desktop:variant-1',
        watermarkUid: 'uid-variant',
        incomingRevision: 2,
        insertedRecordId: 'desktop:variant-1',
      ),
    );
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();

    expect(find.text('同步处理记录'), findsOneWidget);
    expect(find.text('待登记仲裁 · uid-variant · v2'), findsOneWidget);
    expect(find.text('累计'), findsOneWidget);
  });

  testWidgets('renders sync help in settings', (WidgetTester tester) async {
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
        nextRetryAt: DateTime(2026, 6, 17, 12),
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
        lastError: 'HTTP 403 工作区或设备与云端账户不匹配',
        lastRemotePullCursor: '2026-06-16T12:00:00.000Z',
        lastSyncAttemptAt: DateTime.fromMillisecondsSinceEpoch(3000),
        lastSyncSuccessAt: DateTime.fromMillisecondsSinceEpoch(2500),
        lastSyncFailureAt: DateTime.fromMillisecondsSinceEpoch(3500),
      ),
    );
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();
    await tester.drag(find.byType(ListView).last, const Offset(0, -520));
    await tester.pumpAndSettle();

    expect(find.text('同步状态'), findsOneWidget);
    expect(find.text('需恢复账户'), findsOneWidget);
    expect(find.text('账户、设备或工作区授权不一致，请重新登录。'), findsOneWidget);
    expect(find.text('连接失败'), findsWidgets);
    expect(find.text('待同步 0 · 失败 1'), findsOneWidget);
    expect(find.text('下次自动重试'), findsOneWidget);
    expect(find.text('最近尝试'), findsWidgets);
    expect(find.text('最近成功'), findsWidgets);
    expect(find.text('最近失败'), findsOneWidget);
    expect(find.byTooltip('复制同步信息'), findsOneWidget);
    expect(find.textContaining('HTTP 403'), findsWidgets);
    expect(find.text('账户状态需要恢复'), findsOneWidget);
    expect(find.text('重新登录'), findsOneWidget);
    expect(find.text('重试失败'), findsOneWidget);
  });

  testWidgets('renders account identity contract in settings', (
    WidgetTester tester,
  ) async {
    final state = _testAppState(MemoryVaultStore());
    await state.load();
    state.updateCreatorLabel('Alice Creator');
    await state.continueWithAccountPlaceholder(
      accountLabel: 'alice@example.com',
      password: 'password-123',
    );
    await state.completeBaseSetup(creatorLabel: 'Alice Creator');

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();

    expect(find.text('alice@example.com'), findsWidgets);
    expect(find.text('个人空间'), findsWidgets);
    expect(find.textContaining('当前移动设备'), findsWidgets);
    expect(find.text('云同步'), findsWidgets);
    expect(find.textContaining('creator_'), findsNothing);
  });

  testWidgets('renders mobile settings feedback and diagnostics parity', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(
      find.text('问题反馈'),
      220,
      scrollable: _visibleScrollable(),
    );
    await tester.pumpAndSettle();

    expect(find.text('匿名反馈'), findsWidgets);
    expect(find.text('体验改进'), findsWidgets);
    expect(find.text('占用'), findsOneWidget);
    expect(find.text('问题反馈'), findsOneWidget);
    expect(find.text('导出日志'), findsOneWidget);
    expect(find.text('发送反馈'), findsWidgets);
    expect(find.text('Zoro998877'), findsOneWidget);
    expect(find.text('jhx800@163.com'), findsOneWidget);
    expect(find.textContaining('不包含媒体文件、本地路径、文件名或完整作品指纹'), findsOneWidget);
    expect(find.textContaining('本地路径、媒体文件、受保护副本路径不进入云同步或匿名反馈'), findsWidgets);
  });

  testWidgets('renders the split video capability cards in workspace', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.scrollUntilVisible(
      find.text('视频音轨水印'),
      180,
      scrollable: _visibleScrollable(),
    );
    await tester.pumpAndSettle();
    expect(find.text('视频音轨水印'), findsOneWidget);
    await tester.scrollUntilVisible(
      find.text('视频指纹存证与 L3 对象上传入口'),
      240,
      scrollable: _visibleScrollable(),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('视频指纹存证'), findsWidgets);
    expect(find.textContaining('L1 视频音轨水印可在移动端验证'), findsOneWidget);
    expect(find.textContaining('L2 提交存证只生成不可逆 metadata 指纹包'), findsOneWidget);
    expect(find.text('提交 L2 指纹存证'), findsOneWidget);
  });

  testWidgets('free account cannot enable formal cloud sync', (
    WidgetTester tester,
  ) async {
    final state = _testAppState(MemoryVaultStore());
    await state.load();
    await state.continueWithAccountPlaceholder(
      accountLabel: 'free@example.com',
      password: 'password-123',
    );
    await state.completeBaseSetup(creatorLabel: 'Free Creator');

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();

    expect(state.cloudSyncEnabled, isFalse);
    expect(find.textContaining('Creator 起开放正式云同步'), findsOneWidget);
  });

  testWidgets('opens subscription plans from settings', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('查看订阅方案'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('查看订阅方案'));
    await tester.pumpAndSettle();

    expect(find.text('Free / Creator / Studio / Enterprise'), findsOneWidget);
    expect(find.text('Creator'), findsOneWidget);
    expect(find.text('批量队列'), findsWidgets);
    expect(find.text('批量队列是订阅权益，不按本地处理次数扣点。'), findsOneWidget);
  });

  testWidgets('free users see the local batch subscription gate', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('批量').last);
    await tester.pumpAndSettle();

    expect(find.text('Free 可使用单文件写入'), findsOneWidget);
    expect(find.textContaining('Free 不进入文件选择'), findsWidgets);
    expect(find.text('创建队列'), findsNothing);
  });

  testWidgets('creator users can enter the local batch queue preview', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    await store.saveSyncProfile(
      SyncProfile(
        mode: SyncTransportMode.cloud,
        status: SyncConnectionStatus.connected,
        updatedAt: DateTime.fromMillisecondsSinceEpoch(1000),
        entitlementLabel: 'Creator',
        entitlementStatus: EntitlementStatus.active,
        entitlementPlanCode: 'creator',
        entitlementFeatures: const {
          'cloud_sync': true,
          'batch_processing': true,
          'report_export': true,
          'cloud_batch_processing': false,
          'cloud_video_processing': false,
          'priority_queue': false,
          'team_workspace': false,
          'api_access': false,
        },
      ),
    );
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state, creatorLabel: 'Creator User');

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('批量').last);
    await tester.pumpAndSettle();

    expect(find.text('批量队列已开放'), findsOneWidget);
    expect(find.text('批量队列'), findsWidgets);
    expect(find.text('支持图片和音频。音频需满足 30 秒以上规则。'), findsOneWidget);
    expect(find.text('失败可重试'), findsOneWidget);
    expect(find.text('创建队列'), findsOneWidget);
    expect(find.textContaining('Free 不进入文件选择'), findsNothing);
  });

  testWidgets('mobile workspace gates L2 submit and L3 upload by plan', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.scrollUntilVisible(
      find.text('视频指纹存证与 L3 对象上传入口'),
      180,
      scrollable: _visibleScrollable(),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('视频指纹存证'), findsWidgets);
    expect(find.textContaining('当前移动端可查看已同步的视频指纹存证记录'), findsOneWidget);
    expect(find.textContaining('L2 提交需 Creator 云同步权益'), findsOneWidget);
    expect(
      find.textContaining('L3 视频画面盲水印需 Studio / Enterprise 对象上传入口'),
      findsOneWidget,
    );
  });

  testWidgets('local batch shows friendly audio duration failures', (
    WidgetTester tester,
  ) async {
    final now = DateTime.fromMillisecondsSinceEpoch(1000);
    final store = MemoryVaultStore();
    await store.saveSyncProfile(
      SyncProfile(
        mode: SyncTransportMode.cloud,
        status: SyncConnectionStatus.connected,
        updatedAt: now,
        entitlementLabel: 'Creator',
        entitlementStatus: EntitlementStatus.active,
        entitlementPlanCode: 'creator',
        entitlementFeatures: const {'batch_processing': true},
      ),
    );
    await store.saveLocalBatchJob(
      LocalBatchJob(
        id: 'batch-duration',
        status: BatchJobStatus.queued,
        createdAt: now,
        updatedAt: now,
        entitlementPlanCode: 'creator',
        entitlementStatus: EntitlementStatus.active,
        items: [
          LocalBatchItem(
            id: 'item-short',
            jobId: 'batch-duration',
            inputRef: 'short.mp3',
            fileName: 'short.mp3',
            mediaKind: BatchMediaKind.audio,
            status: BatchItemStatus.failed,
            attempts: 1,
            createdAt: now,
            updatedAt: now,
            lastError: '当前音频短于 30 秒，暂不生成保护副本',
          ),
          LocalBatchItem(
            id: 'item-unknown',
            jobId: 'batch-duration',
            inputRef: 'unknown.m4a',
            fileName: 'unknown.m4a',
            mediaKind: BatchMediaKind.audio,
            status: BatchItemStatus.failed,
            attempts: 1,
            createdAt: now,
            updatedAt: now,
            lastError: '无法确认音频时长，暂不生成保护副本',
          ),
        ],
      ),
    );
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state, creatorLabel: 'Creator User');

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('批量').last);
    await tester.pumpAndSettle();

    expect(find.textContaining('音频时长不足 30 秒，未生成保护副本'), findsOneWidget);
    expect(find.textContaining('请选择 30 秒以上的完整音频作品后重试'), findsOneWidget);
    expect(find.textContaining('无法确认音频时长，未生成保护副本'), findsOneWidget);
    expect(find.textContaining('请更换可识别时长的完整音频文件后重试'), findsOneWidget);
  });

  testWidgets('hides temporary direct connection controls in settings', (
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
    final state = _testAppState(store);
    await state.load();
    state.setSyncTransportMode(SyncTransportMode.lanDebug);
    await _markOnboardingComplete(store, state);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('设置').last);
    await tester.pumpAndSettle();
    await tester.drag(find.byType(ListView).last, const Offset(0, -720));
    await tester.pumpAndSettle();

    expect(find.text('临时直连'), findsNothing);
    expect(find.text('LAN 调试地址'), findsNothing);
    expect(find.text('调试配对码'), findsNothing);
    expect(find.text('联调检查'), findsNothing);
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
        creatorDisplayName: 'Alice Creator',
        trustedTimeStatus: '未记录',
        thirdPartyVerificationStatus: '未记录',
        sha256: 'abcdef1234567890',
        parentWatermarkUid: 'uid-parent',
        rewriteReason: 'authorized rewrite',
        extractedTimestamp: 1781924995,
        extractedDeviceIdHex: '090a0b0c',
        extractedFileHashHex: 'hash',
        writeVerificationStatus: WriteVerificationStatus.verified,
        writeVerificationMessage: '已回读验证版权编号，保护副本可取证。',
        writeVerificationAt: DateTime.parse('2026-06-20T03:09:55Z'),
        source: VaultRecordSource.write,
        syncStatus: SyncStatus.synced,
        createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
      ),
    );
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('版权库').last);
    await tester.pumpAndSettle();
    await tester.drag(find.byType(ListView).first, const Offset(0, -520));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('vault-record-record-1')));
    await tester.pumpAndSettle();

    expect(find.text('版权信息'), findsOneWidget);
    expect(find.text('创作者身份'), findsOneWidget);
    expect(find.text('Alice Creator'), findsOneWidget);
    expect(find.text('第三方验证 / 可信时间'), findsOneWidget);
    expect(find.text('第三方验证'), findsWidgets);
    expect(find.text('可信时间'), findsWidgets);
    await tester.drag(find.byType(ListView).last, const Offset(0, -320));
    await tester.pumpAndSettle();
    expect(find.text('文件指纹'), findsOneWidget);
    expect(find.text('第 3 次'), findsOneWidget);
    expect(find.text('uid-audio'), findsOneWidget);

    await tester.scrollUntilVisible(
      find.text('本地记录'),
      220,
      scrollable: _bottomSheetScrollable(),
    );
    await tester.pumpAndSettle();

    expect(find.text('写入后验证信息'), findsOneWidget);
    expect(find.text('090a0b0c'), findsOneWidget);
    expect(find.text('本地记录'), findsOneWidget);
    expect(find.text('复制存证摘要'), findsOneWidget);
  });

  testWidgets('opens synced video notary record details', (
    WidgetTester tester,
  ) async {
    final store = MemoryVaultStore();
    await store.upsertRecord(
      VaultRecord(
        id: 'video-record-1',
        kind: WatermarkAssetKind.video,
        title: 'demo-video.mp4',
        watermarkUid: 'uid-video',
        revision: 1,
        sha256: 'sha256:source',
        videoNotaryId: 'vfn_123',
        videoNotaryAt: DateTime.parse('2026-06-19T08:01:00Z'),
        videoNotaryReceiptSignature: 'sig_abc',
        videoNotaryUsageLedgerId: 'usage_123',
        videoFingerprintRoot: 'sha256:fingerprint-root',
        videoBundleSha256: 'sha256:bundle',
        videoBundleBytes: 4096,
        videoBundleSceneCount: 8,
        videoBundleElapsedMs: 1234,
        videoFrameSamplePolicy: '8 evenly spaced frames',
        source: VaultRecordSource.write,
        syncStatus: SyncStatus.synced,
        createdAt: DateTime.parse('2026-06-19T08:00:00Z'),
      ),
    );
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state);

    await tester.pumpWidget(HiddenShieldApp(appState: state));
    await tester.pumpAndSettle();

    await tester.tap(find.text('版权库').last);
    await tester.pumpAndSettle();

    expect(find.textContaining('视频指纹存证: vfn_123'), findsOneWidget);

    final videoRecordCard = find.byKey(
      const ValueKey('vault-record-video-record-1'),
    );
    await tester.ensureVisible(videoRecordCard);
    await tester.pumpAndSettle();
    await tester.tap(videoRecordCard);
    await tester.pumpAndSettle();

    await tester.scrollUntilVisible(
      find.text('视频指纹存证'),
      220,
      scrollable: _bottomSheetScrollable(),
    );
    await tester.pumpAndSettle();

    expect(find.text('视频指纹存证'), findsWidgets);
    expect(find.text('存证编号'), findsOneWidget);
    expect(find.text('vfn_123'), findsWidgets);
    expect(find.text('收据签名'), findsNothing);
    expect(find.text('sig_abc'), findsNothing);
    expect(find.text('用量流水'), findsNothing);
    expect(find.text('usage_123'), findsNothing);
    await tester.scrollUntilVisible(
      find.text('指纹根'),
      220,
      scrollable: _bottomSheetScrollable(),
    );
    await tester.pumpAndSettle();

    expect(find.text('指纹根'), findsOneWidget);
    expect(find.text('sha256:fingerprint-root'), findsOneWidget);
    expect(find.text('指纹包摘要'), findsOneWidget);
    expect(find.text('sha256:bundle'), findsOneWidget);
    expect(find.text('指纹包大小'), findsNothing);
    expect(find.text('4096 bytes'), findsNothing);
    expect(find.text('采样策略'), findsOneWidget);
    expect(find.text('8 evenly spaced frames'), findsOneWidget);
    expect(find.textContaining('bundle.json'), findsNothing);
    expect(find.textContaining('D:\\'), findsNothing);
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
    final state = _testAppState(store);
    await state.load();
    await _markOnboardingComplete(store, state);

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

    await tester.tap(find.byKey(const ValueKey('vault-filter-验证')));
    await tester.pumpAndSettle();

    expect(find.text('suspect.wav'), findsOneWidget);
    expect(find.text('显示 1 / 2 条记录'), findsOneWidget);
  });
}

Future<MobileAppState> _readyAppState({MemoryVaultStore? store}) async {
  final vaultStore = store ?? MemoryVaultStore();
  final state = _testAppState(vaultStore);
  await state.load();
  await _markOnboardingComplete(vaultStore, state);
  return state;
}

Future<void> _markOnboardingComplete(
  MemoryVaultStore store,
  MobileAppState state, {
  String creatorLabel = '测试创作者',
}) async {
  final profile = state.syncProfile;
  await store.saveSyncProfile(
    profile.copyWith(
      accountId: profile.accountId ?? 'acct_test',
      accountLabel: profile.accountLabel ?? 'tester@example.com',
      authToken: profile.authToken ?? 'test-token',
      refreshToken: profile.refreshToken ?? 'test-refresh-token',
      workspaceId: profile.workspaceId ?? 'ws_test',
      workspaceName: profile.workspaceName ?? '个人空间',
      deviceId: profile.deviceId ?? 'dev_test',
      deviceName: profile.deviceName ?? '当前移动设备',
      devicePlatform: profile.devicePlatform ?? 'test',
      deviceRegistered: true,
      creatorProfileId: profile.creatorProfileId ?? 'creator_test',
      creatorDisplayName: creatorLabel,
      creatorProfileSynced: true,
      onboardingCompleted: true,
      updatedAt: DateTime.fromMillisecondsSinceEpoch(1000),
    ),
  );
  state.updateCreatorLabel(creatorLabel);
  await state.load();
}

Finder _visibleScrollable() {
  return find
      .byWidgetPredicate(
        (widget) =>
            widget is Scrollable &&
            widget.physics is! NeverScrollableScrollPhysics,
      )
      .first;
}

Finder _bottomSheetScrollable() {
  return find
      .descendant(
        of: find.byType(BottomSheet),
        matching: find.byWidgetPredicate((widget) => widget is Scrollable),
      )
      .first;
}

MobileAppState _testAppState(VaultStore vaultStore) {
  return MobileAppState(
    vaultStore: vaultStore,
    offlineLicenseManager: OfflineLicenseManager(
      secureStore: _MemoryOfflineLicenseSecureStore(),
      platform: 'android',
      appVersion: '1.0.0',
    ),
  );
}

class _MemoryOfflineLicenseSecureStore implements OfflineLicenseSecureStore {
  final Map<String, String> _values = {};

  @override
  Future<void> delete(String key) async {
    _values.remove(key);
  }

  @override
  Future<String?> read(String key) async => _values[key];

  @override
  Future<void> write(String key, String value) async {
    _values[key] = value;
  }
}
