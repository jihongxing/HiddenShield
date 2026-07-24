import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:hidden_shield_mobile/app/mobile_anonymous_feedback.dart';
import 'package:hidden_shield_mobile/app/mobile_app_state.dart';
import 'package:hidden_shield_mobile/app/mobile_time_attestation.dart';
import 'package:hidden_shield_mobile/bridge/watermark_models.dart';
import 'package:hidden_shield_mobile/storage/vault_store.dart';
import 'package:hidden_shield_mobile/sync/cloud_account_client.dart';
import 'package:hidden_shield_mobile/sync/sync_transport.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

const _testSeed = WatermarkPayloadSeed(
  creatorIdentity: 'creator-test',
  deviceIdentity: 'device-test',
  mediaBytes: [1, 2, 3, 4],
  timestamp: 1781924995,
);

void main() {
  test('loads existing vault records from the store', () async {
    final store = MemoryVaultStore();
    await store.upsertRecord(
      VaultRecord(
        id: 'existing-record',
        kind: WatermarkAssetKind.image,
        title: 'existing.png',
        watermarkUid: 'uid-existing',
        revision: 1,
        source: VaultRecordSource.write,
        syncStatus: SyncStatus.pending,
        createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
      ),
    );

    final state = MobileAppState(vaultStore: store);
    await state.load();

    expect(state.isLoaded, isTrue);
    expect(state.records, hasLength(1));
    expect(state.records.single.watermarkUid, 'uid-existing');
    expect(state.syncQueue, isEmpty);
  });

  test('persists write results and queues them for desktop sync', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();
    await state.completeBaseSetup(creatorLabel: 'Alice Creator');

    final trustedTimeAt = DateTime.utc(2026, 6, 20, 2, 4, 35);
    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.audio,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-audio',
        revision: 1,
        sha256: 'abc123',
        seed: _testSeed,
        processTimeMs: 1234,
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'song.wav',
      allowRewrite: false,
      trustedTimeAttestation: MobileTrustedTimeAttestation(
        trustedTimeStatus: '已记录网络授时',
        trustedTimeSource: 'https://www.aliyun.com',
        trustedTimeAt: trustedTimeAt,
        thirdPartyVerificationStatus: '已记录网络授时',
        thirdPartyVerificationProvider: 'www.aliyun.com',
        thirdPartyVerificationPath: 'HTTP Date 响应头',
      ),
    );

    final persisted = await store.loadRecords();
    expect(persisted, hasLength(1));
    expect(persisted.single.title, 'song.wav');
    expect(persisted.single.syncStatus, SyncStatus.pending);
    expect(
      persisted.single.writeVerificationStatus,
      WriteVerificationStatus.verified,
    );
    expect(persisted.single.writeVerificationMessage, '已回读验证版权编号，保护副本可取证。');
    expect(persisted.single.writeVerificationAt, isNotNull);
    expect(persisted.single.creatorDisplayName, 'Alice Creator');
    expect(persisted.single.trustedTimeStatus, '已记录网络授时');
    expect(persisted.single.trustedTimeSource, 'https://www.aliyun.com');
    expect(persisted.single.trustedTimeAt, trustedTimeAt);
    expect(persisted.single.thirdPartyVerificationStatus, '已记录网络授时');
    expect(persisted.single.thirdPartyVerificationProvider, 'www.aliyun.com');
    expect(persisted.single.thirdPartyVerificationPath, 'HTTP Date 响应头');
    expect(persisted.single.extractedTimestamp, _testSeed.timestamp);
    expect(persisted.single.extractedDeviceIdHex, 'device-test');

    final queue = await store.loadSyncQueue();
    expect(queue, hasLength(1));
    expect(queue.single.recordId, persisted.single.id);
    expect(queue.single.operation, SyncQueueOperation.upsertVaultRecord);
    expect(queue.single.status, SyncQueueItemStatus.pending);
    final payload =
        jsonDecode(queue.single.payloadJson) as Map<String, Object?>;
    expect(payload['write_verification_status'], 'verified');
    expect(payload['write_verification_message'], '已回读验证版权编号，保护副本可取证。');
    expect(payload['write_verification_at'], isA<String>());
    expect(payload['creator_display_name'], 'Alice Creator');
    expect(payload['trusted_time_status'], '已记录网络授时');
    expect(payload['trusted_time_source'], 'https://www.aliyun.com');
    expect(payload['trusted_time_at'], trustedTimeAt.toIso8601String());
    expect(payload['third_party_verification_status'], '已记录网络授时');
    expect(payload['third_party_verification_provider'], 'www.aliyun.com');
    expect(payload['third_party_verification_path'], 'HTTP Date 响应头');
    expect(payload.keys, everyElement(isIn(vaultRecordSyncPayloadKeys)));
    expect(payload.containsKey('output_ref'), isFalse);
    expect(payload.containsKey('local_path'), isFalse);
    expect(payload.containsKey('input_ref'), isFalse);
    expect(payload.containsKey('protected_media_path'), isFalse);
    expect(state.pendingSyncQueueCount, 1);
  });

  test(
    'rejects web preview write results from formal vault and sync',
    () async {
      final store = MemoryVaultStore();
      final state = MobileAppState(vaultStore: store);
      await state.load();

      expect(
        () => state.addWriteResult(
          result: const WatermarkWriteResult(
            kind: WatermarkAssetKind.image,
            bytes: [1, 2, 3],
            watermarkUid: 'HS-00010002-00030004-00050006-00070008',
            revision: 1,
            sha256: 'preview-hash',
            seed: _testSeed,
            isProductionWatermark: false,
            processTimeMs: 1234,
            verification: WatermarkWriteVerification(
              verified: true,
              watermarkUid: 'HS-00010002-00030004-00050006-00070008',
              revision: 1,
              message: 'preview',
            ),
          ),
          fileName: 'preview.png',
          allowRewrite: false,
        ),
        throwsA(
          isA<StateError>().having(
            (error) => error.message,
            'message',
            'Web 预览结果不能写入正式版权库或云同步队列。',
          ),
        ),
      );

      expect(await store.loadRecords(), isEmpty);
      expect(await store.loadSyncQueue(), isEmpty);
    },
  );

  test('rejects web preview read results from formal vault and sync', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    expect(
      () => state.addReadResult(
        result: const WatermarkReadResult(
          kind: WatermarkAssetKind.audio,
          watermarkUid: 'HS-00010002-00030004-00050006-00070008',
          revision: 1,
          timestamp: 1781924995,
          deviceIdHex: 'preview-device',
          fileHashHex: 'abcd',
          isProductionWatermark: false,
        ),
        fileName: 'preview.wav',
      ),
      throwsA(
        isA<StateError>().having(
          (error) => error.message,
          'message',
          'Web 预览验证结果不能写入正式版权库或云同步队列。',
        ),
      ),
    );

    expect(await store.loadRecords(), isEmpty);
    expect(await store.loadSyncQueue(), isEmpty);
  });

  test('parses HTTP Date header for mobile trusted time', () {
    expect(
      parseHttpDateHeader('Sat, 20 Jun 2026 02:04:35 GMT'),
      DateTime.utc(2026, 6, 20, 2, 4, 35),
    );
  });

  test('requests mobile trusted time from backend proxy first', () async {
    final client = MobileTrustedTimeClient(
      backendEndpoint: Uri.parse('http://127.0.0.1:43188/v1/trusted-time'),
      httpClient: MockClient((request) async {
        expect(request.url.path, '/v1/trusted-time');
        return http.Response(
          jsonEncode({
            'status': '已记录网络授时',
            'source': 'https://freetsa.org/tsr',
            'trustedTimeAt': '2026-06-20T02:57:09Z',
            'thirdPartyVerificationStatus': '已记录网络授时',
            'thirdPartyVerificationProvider': 'freetsa.org',
            'verificationPath': 'HiddenShield 后端 HTTP Date',
          }),
          200,
          headers: {'content-type': 'application/json'},
        );
      }),
      endpoints: const [],
    );

    final attestation = await client.request();

    expect(attestation, isNotNull);
    expect(attestation!.trustedTimeStatus, '已记录网络授时');
    expect(attestation.trustedTimeSource, 'https://freetsa.org/tsr');
    expect(attestation.trustedTimeAt, DateTime.utc(2026, 6, 20, 2, 57, 9));
    expect(attestation.thirdPartyVerificationProvider, 'freetsa.org');
    expect(attestation.thirdPartyVerificationPath, 'HiddenShield 后端 HTTP Date');
  });

  test('records verified write results in the mobile usage ledger', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    final result = const WatermarkWriteResult(
      kind: WatermarkAssetKind.audio,
      bytes: [1, 2, 3, 4],
      watermarkUid: 'uid-audio',
      revision: 1,
      sha256: 'abc123',
      seed: _testSeed,
      processTimeMs: 1234,
      verification: WatermarkWriteVerification(
        verified: true,
        watermarkUid: 'verified-uid',
        revision: 1,
        message: '已回读验证版权编号，保护副本可取证。',
      ),
    );

    final record = state.addWriteResult(
      result: result,
      fileName: 'song.wav',
      allowRewrite: false,
    );
    await state.appendUsageForWriteResult(
      result: result,
      vaultRecordId: record.id,
      pipelineId: 'batch-1/item-1',
    );

    expect(state.usageSummary.totalUnits, 1);
    expect(state.usageSummary.audioUnits, 1);
    expect(state.usageSummary.imageUnits, 0);
    expect(state.usageSummary.lastFeatureName, 'watermark_audio');

    final reloaded = MobileAppState(vaultStore: store);
    await reloaded.load();
    expect(reloaded.usageSummary.totalEvents, 1);
    expect(reloaded.usageSummary.totalUnits, 1);
    expect(reloaded.usageSummary.audioUnits, 1);
  });

  test(
    'persists anonymous feedback settings and exports safe diagnostics',
    () async {
      final store = MemoryVaultStore();
      final feedbackClient = MobileAnonymousFeedbackClient(
        httpClient: MockClient(
          (_) async => http.Response('service unavailable', 503),
        ),
        endpoint: Uri.parse(
          'http://127.0.0.1:43188/v1/anonymous-feedback/batches',
        ),
      );
      final state = MobileAppState(
        vaultStore: store,
        anonymousFeedbackClient: feedbackClient,
      );
      await state.load();

      await state.setAnonymousFeedbackEnabled(true);
      await state.setExperienceImprovementEnabled(false);
      final result = await state.flushAnonymousFeedbackQueue();

      expect(result.remainingEvents, 1);
      expect(state.anonymousFeedbackStatus.queuedEvents, 1);
      expect(state.anonymousFeedbackStatus.lastAttemptAt, isNotNull);

      final log = state.exportSafeDiagnosticLog();
      expect(log, contains('HiddenShield 移动端安全诊断日志'));
      expect(log, contains('不上传原始媒体、加水印媒体、本地路径、文件名或完整作品指纹'));
      expect(log, isNot(contains('D:\\')));
      expect(log, isNot(contains('Abstract4.jpg')));

      final reloaded = MobileAppState(
        vaultStore: store,
        anonymousFeedbackClient: feedbackClient,
      );
      await reloaded.load();
      expect(reloaded.anonymousFeedbackEnabled, isTrue);
      expect(reloaded.experienceImprovementEnabled, isFalse);
      expect(reloaded.anonymousFeedbackStatus.queuedEvents, 1);
    },
  );

  test(
    'formal report draft includes video notary fields as report usage',
    () async {
      final store = MemoryVaultStore();
      await store.saveSyncProfile(
        SyncProfile.localOnly().copyWith(
          entitlementStatus: EntitlementStatus.active,
          entitlementPlanCode: 'creator',
          entitlementLabel: 'Creator',
          entitlementFeatures: const {'report_export': true},
          updatedAt: DateTime.now(),
        ),
      );
      final state = MobileAppState(vaultStore: store);
      await state.load();
      final record = VaultRecord(
        id: 'video-record-1',
        kind: WatermarkAssetKind.video,
        title: 'demo-video.mp4',
        watermarkUid: 'uid-video',
        revision: 1,
        source: VaultRecordSource.write,
        syncStatus: SyncStatus.synced,
        createdAt: DateTime.parse('2026-06-19T08:00:00Z'),
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
      );

      final draft = await state.buildFormalReportDraft(record);

      expect(draft.markdown, contains('## 视频指纹存证'));
      expect(draft.markdown, contains('## 结构化字段'));
      expect(draft.markdown, contains('- trusted_time_status'));
      expect(draft.markdown, contains('## 可信时间'));
      expect(draft.markdown, contains('- 第三方验证: 未记录'));
      expect(draft.markdown, contains('- 可信时间: 未记录'));
      expect(draft.markdown, contains('- 存证编号: vfn_123'));
      expect(draft.markdown, contains('- 收据签名: sig_abc'));
      expect(draft.markdown, contains('- 用量流水: usage_123'));
      expect(draft.markdown, contains('- 指纹根: sha256:fingerprint-root'));
      expect(draft.markdown, contains('- 指纹包摘要: sha256:bundle'));
      expect(draft.markdown, isNot(contains('bundle.json')));
      expect(draft.markdown, isNot(contains('D:\\')));
      expect(state.usageSummary.lastFeatureName, 'report_export');
      expect(state.usageSummary.videoUnits, 0);
      expect(state.usageSummary.totalEvents, 1);
    },
  );

  test(
    'mobile L2 video fingerprint notary posts metadata bundle and queues vault record',
    () async {
      final store = MemoryVaultStore();
      await store.saveSyncProfile(
        SyncProfile.localOnly().copyWith(
          accountId: 'acct-l2',
          authToken: 'access-token-l2',
          workspaceId: 'ws-l2',
          creatorProfileId: 'creator-l2',
          cloudBaseUrl: 'https://api.hiddenshield.test',
          entitlementStatus: EntitlementStatus.active,
          entitlementPlanCode: 'creator',
          entitlementLabel: 'Creator',
          entitlementFeatures: const {'cloud_sync': true},
          updatedAt: DateTime.fromMillisecondsSinceEpoch(1000),
        ),
      );
      Map<String, Object?>? notaryRequest;
      final cloudClient = CloudAccountClient(
        baseUrl: 'https://api.hiddenshield.test',
        client: MockClient((request) async {
          expect(request.url.path, '/v1/video-fingerprints/notaries');
          expect(request.headers['authorization'], 'Bearer access-token-l2');
          notaryRequest = jsonDecode(request.body) as Map<String, Object?>;
          final manifest =
              notaryRequest!['uploadManifest'] as Map<String, Object?>;
          final items = manifest['items'] as List<Object?>;
          final item = items.single as Map<String, Object?>;
          expect(
            notaryRequest!['schemaVersion'],
            'video_fingerprint_notary_request_v1',
          );
          expect(
            notaryRequest!['frameSamplePolicy'],
            'mobile_metadata_probe_v1',
          );
          expect(
            notaryRequest!['fingerprintSchemaVersion'],
            'mobile_metadata_fingerprint_v1',
          );
          expect(manifest['containsOriginalVideo'], isFalse);
          expect(manifest['containsWatermarkedVideo'], isFalse);
          expect(manifest['containsLocalPaths'], isFalse);
          expect(item['kind'], 'mobile_video_fingerprint_metadata');
          expect(item.containsKey('storageRef'), isFalse);
          return http.Response.bytes(
            utf8.encode(
              jsonEncode({
                'schemaVersion': 'video_fingerprint_notary_receipt_v1',
                'notaryId': 'vfn_mobile_l2',
                'watermarkUid': notaryRequest!['watermarkUid'],
                'sourceHash': notaryRequest!['sourceHash'],
                'fingerprintRoot': notaryRequest!['fingerprintRoot'],
                'notarizedAt': '2026-07-02T08:00:00Z',
                'serverReceiptSignature': 'mock_server_signature_l2',
                'usageLedgerId': 'usage_l2',
              }),
            ),
            200,
            headers: const {'content-type': 'application/json; charset=utf-8'},
          );
        }),
      );
      final state = MobileAppState(
        vaultStore: store,
        cloudAccountClient: cloudClient,
      );
      await state.load();
      await state.completeBaseSetup(creatorLabel: 'Mobile L2 Creator');

      final record = await state.createL2VideoFingerprintNotaryFromBytes(
        bytes: const [0, 1, 2, 3, 4, 5, 6, 7],
        fileName: 'mobile-l2.mp4',
        durationMs: 12_000,
        width: 1280,
        height: 720,
        frameCount: 24,
      );

      expect(record.kind, WatermarkAssetKind.video);
      expect(record.videoNotaryId, 'vfn_mobile_l2');
      expect(record.videoFingerprintRoot, notaryRequest!['fingerprintRoot']);
      expect(record.videoBundleSha256, startsWith('sha256:'));
      expect(record.videoBundleSceneCount, 8);
      expect(record.videoFrameSamplePolicy, 'mobile_metadata_probe_v1');
      expect(record.outputStrategy, 'mobile_video_fingerprint_notary');
      final persisted = await store.loadRecords();
      expect(persisted.single.videoNotaryId, 'vfn_mobile_l2');
      final queue = await store.loadSyncQueue();
      expect(queue, hasLength(1));
      final payload =
          jsonDecode(queue.single.payloadJson) as Map<String, Object?>;
      expect(payload['video_notary_id'], 'vfn_mobile_l2');
      expect(payload['video_frame_sample_policy'], 'mobile_metadata_probe_v1');
      expect(payload.containsKey('local_path'), isFalse);
      expect(payload.containsKey('originalVideoPath'), isFalse);
      expect(payload.containsKey('storageRef'), isFalse);
    },
  );

  test(
    'free user can export a purchased single-record report without report_export',
    () async {
      final store = MemoryVaultStore();
      await store.saveSyncProfile(
        SyncProfile.localOnly().copyWith(
          accountId: 'acct-report',
          authToken: 'access-token',
          workspaceId: 'ws-report',
          creatorProfileId: 'creator-report',
          cloudBaseUrl: 'https://api.hiddenshield.test',
          entitlementStatus: EntitlementStatus.free,
          entitlementPlanCode: 'free',
          entitlementLabel: '免费版',
          entitlementFeatures: const {'report_export': false},
          updatedAt: DateTime.now(),
        ),
      );
      final record = _vaultRecord(
        id: 'record-report-1',
        watermarkUid: 'uid-report',
        revision: 1,
        sha256: 'hash-report',
      );
      await store.upsertRecord(record);
      final cloudClient = CloudAccountClient(
        baseUrl: 'https://api.hiddenshield.test',
        client: MockClient((request) async {
          if (request.url.path == '/v1/billing/report-purchase-sessions') {
            final body = jsonDecode(request.body) as Map<String, Object?>;
            expect(body['productCode'], 'rights_evidence_pack_single');
            expect(body['vaultRecordId'], 'record-report-1');
            return http.Response.bytes(
              utf8.encode(
                jsonEncode({
                  'paymentSessionId': 'rpt_pay_sess_1',
                  'provider': 'fixture',
                  'providerOrderId': 'fixture_report_order_1',
                  'productCode': 'rights_evidence_pack_single',
                  'priceCents': 4990,
                  'currency': 'CNY',
                  'paymentAction': {
                    'type': 'qr_code',
                    'qrCodeUrl': 'fixture://pay/report',
                    'h5Url': null,
                  },
                  'expiresAt': '2026-06-25T00:15:00Z',
                }),
              ),
              200,
              headers: const {
                'content-type': 'application/json; charset=utf-8',
              },
            );
          }
          if (request.url.path ==
              '/v1/billing/report-purchase-sessions/rpt_pay_sess_1') {
            return http.Response.bytes(
              utf8.encode(
                jsonEncode({
                  'paymentSessionId': 'rpt_pay_sess_1',
                  'provider': 'fixture',
                  'providerOrderId': 'fixture_report_order_1',
                  'status': 'created',
                  'productCode': 'rights_evidence_pack_single',
                  'priceCents': 4990,
                  'currency': 'CNY',
                  'vaultRecordId': 'record-report-1',
                  'expiresAt': '2026-06-25T00:15:00Z',
                  'lastCheckedAt': null,
                  'nextCheckAfter': null,
                  'checkAttempts': 0,
                  'grant': null,
                }),
              ),
              200,
              headers: const {
                'content-type': 'application/json; charset=utf-8',
              },
            );
          }
          if (request.url.path ==
              '/v1/billing/report-purchase-sessions/rpt_pay_sess_1/reconcile') {
            return http.Response.bytes(
              utf8.encode(
                jsonEncode({
                  'paymentSessionId': 'rpt_pay_sess_1',
                  'status': 'succeeded',
                  'message': '支付已确认，报告授权已生效。',
                  'grant': {
                    'grantId': 'rpt_grant_1',
                    'accountId': 'acct-report',
                    'workspaceId': 'ws-report',
                    'creatorProfileId': 'creator-report',
                    'vaultRecordId': 'record-report-1',
                    'productCode': 'rights_evidence_pack_single',
                    'priceCents': 4990,
                    'currency': 'CNY',
                    'status': 'active',
                    'grantedAt': '2026-06-25T00:00:00Z',
                    'revokedAt': null,
                  },
                }),
              ),
              200,
              headers: const {
                'content-type': 'application/json; charset=utf-8',
              },
            );
          }
          return http.Response('not found', 404);
        }),
      );
      final state = MobileAppState(
        vaultStore: store,
        cloudAccountClient: cloudClient,
      );
      await state.load();

      expect(state.canExportFormalReports, isFalse);
      expect(state.canExportFormalReportForRecord(record), isFalse);

      final session = await state.createReportPurchaseSession(
        record: record,
        productCode: 'rights_evidence_pack_single',
      );
      expect(session?.priceCents, 4990);
      final granted = await state.reconcileReportPurchaseSession(
        paymentSessionId: session!.paymentSessionId,
      );

      expect(granted, isTrue);
      expect(state.canExportFormalReportForRecord(record), isTrue);
      final draft = await state.buildFormalReportDraft(record);
      expect(draft.markdown, contains('HiddenShield 正式版权报告'));
      expect(state.syncProfile.entitlementFeatures['report_export'], isFalse);

      final reloaded = MobileAppState(vaultStore: store);
      await reloaded.load();
      expect(reloaded.canExportFormalReportForRecord(record), isTrue);
    },
  );

  test('persists and reloads local batch jobs', () async {
    final store = MemoryVaultStore();
    final createdAt = DateTime.fromMillisecondsSinceEpoch(1000);
    final updatedAt = DateTime.fromMillisecondsSinceEpoch(2000);
    final state = MobileAppState(vaultStore: store);
    await state.load();

    await state.saveLocalBatchJob(
      LocalBatchJob(
        id: 'batch-1',
        status: BatchJobStatus.queued,
        createdAt: createdAt,
        updatedAt: updatedAt,
        entitlementPlanCode: 'creator',
        entitlementStatus: EntitlementStatus.active,
        items: [
          LocalBatchItem(
            id: 'item-1',
            jobId: 'batch-1',
            inputRef: 'cover.png',
            fileName: 'cover.png',
            mediaKind: BatchMediaKind.image,
            status: BatchItemStatus.verified,
            attempts: 0,
            createdAt: createdAt,
            updatedAt: updatedAt,
            vaultRecordId: 'record-1',
            writeVerificationStatus: WriteVerificationStatus.verified,
            writeVerificationMessage: '完成后验证已通过',
          ),
        ],
      ),
    );

    final reloaded = MobileAppState(vaultStore: store);
    await reloaded.load();

    expect(reloaded.localBatchJobs, hasLength(1));
    expect(reloaded.latestLocalBatchJob?.id, 'batch-1');
    expect(reloaded.latestLocalBatchJob?.items.single.fileName, 'cover.png');
    expect(
      reloaded.latestLocalBatchJob?.items.single.status,
      BatchItemStatus.verified,
    );
    expect(
      reloaded.latestLocalBatchJob?.items.single.vaultRecordId,
      'record-1',
    );
    expect(
      reloaded.latestLocalBatchJob?.items.single.writeVerificationStatus,
      WriteVerificationStatus.verified,
    );
  });

  test('persists mobile rewrite lineage for write results', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.image,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-rewrite',
        revision: 2,
        sha256: 'hash-rewrite',
        seed: _testSeed,
        processTimeMs: 1234,
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'rewrite.png',
      allowRewrite: true,
      rewriteReason: 'mobile explicit rewrite',
      parentWatermarkUid: 'uid-parent',
      revision: 4,
    );

    final persisted = await store.loadRecords();
    expect(persisted.single.revision, 4);
    expect(persisted.single.parentWatermarkUid, 'uid-parent');
    expect(persisted.single.rewriteReason, 'mobile explicit rewrite');

    final queue = await store.loadSyncQueue();
    expect(queue.single.payloadJson, contains('"revision":4'));
    expect(queue.single.payloadJson, contains('uid-parent'));
    expect(queue.single.payloadJson, contains('mobile explicit rewrite'));
  });

  test('persists verify results and queues evidence records', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    state.addReadResult(
      result: const WatermarkReadResult(
        kind: WatermarkAssetKind.image,
        watermarkUid: 'uid-image',
        revision: 2,
        timestamp: 123,
        deviceIdHex: 'device',
        fileHashHex: 'hash',
        parentWatermarkUid: 'uid-parent',
        rewriteReason: 'authorized rewrite',
        payloadProtocolVersion: 2,
        payloadBytesLength: 119,
        watermarkIdIssueMode: 'server_confirmed',
        payloadAuthStatus: 'verified',
      ),
      fileName: 'suspect.png',
    );

    final persisted = await store.loadRecords();
    expect(persisted, hasLength(1));
    expect(persisted.single.source, VaultRecordSource.verify);
    expect(persisted.single.syncStatus, SyncStatus.pending);
    expect(persisted.single.extractedTimestamp, 123);
    expect(persisted.single.extractedDeviceIdHex, 'device');
    expect(persisted.single.extractedFileHashHex, 'hash');
    expect(persisted.single.payloadProtocolVersion, 2);
    expect(persisted.single.payloadBytesLength, 119);
    expect(persisted.single.watermarkIdIssueMode, 'server_confirmed');
    expect(persisted.single.watermarkIdRegistryStatus, 'server_confirmed');
    expect(persisted.single.payloadAuthStatus, 'verified');

    final queue = await store.loadSyncQueue();
    expect(queue, hasLength(1));
    expect(queue.single.operation, SyncQueueOperation.upsertEvidenceRecord);
    expect(queue.single.payloadJson, contains('uid-parent'));
    expect(queue.single.payloadJson, contains('extracted_timestamp'));
    expect(queue.single.payloadJson, contains('device'));
    expect(queue.single.payloadJson, contains('hash'));
    expect(queue.single.payloadJson, contains('"payload_protocol_version":2'));
    expect(queue.single.payloadJson, contains('"payload_bytes_length":119'));
    expect(queue.single.payloadJson, contains('server_confirmed'));
    expect(
      queue.single.payloadJson,
      contains('"payload_auth_status":"verified"'),
    );
  });

  test('syncs pending queue items with the local mock transport', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: const LocalMockSyncTransport(),
    );
    await state.load();

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.image,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-sync',
        revision: 1,
        sha256: 'hash',
        seed: _testSeed,
        processTimeMs: 1234,
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'work.png',
      allowRewrite: false,
    );

    await state.syncPendingQueue();

    final queue = await store.loadSyncQueue();
    expect(queue.single.status, SyncQueueItemStatus.synced);
    expect(queue.single.attempts, 1);
    expect(queue.single.lastError, isNull);
    expect(state.pendingSyncQueueCount, 0);
    expect(state.syncProfile.lastSyncAttemptAt, isNotNull);
    expect(state.syncProfile.lastSyncSuccessAt, isNotNull);
    expect(state.syncProfile.lastSyncFailureAt, isNull);
    expect(state.syncProfile.status, SyncConnectionStatus.connected);
  });

  test('marks failed sync attempts and keeps retry metadata', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: const LocalMockSyncTransport(shouldFail: true),
    );
    await state.load();

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.audio,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-fail',
        revision: 1,
        sha256: 'hash',
        seed: _testSeed,
        processTimeMs: 1234,
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'fail.wav',
      allowRewrite: false,
    );

    await state.syncPendingQueue();

    final queue = await store.loadSyncQueue();
    expect(queue.single.status, SyncQueueItemStatus.failed);
    expect(queue.single.attempts, 1);
    expect(queue.single.lastError, 'local mock sync failed');
    expect(queue.single.nextRetryAt, isNotNull);
    expect(queue.single.nextRetryAt!.isAfter(DateTime.now()), isTrue);
    expect(state.failedSyncQueueCount, 1);
    expect(state.syncProfile.lastSyncAttemptAt, isNotNull);
    expect(state.syncProfile.lastSyncSuccessAt, isNull);
    expect(state.syncProfile.lastSyncFailureAt, isNotNull);
    expect(state.syncProfile.status, SyncConnectionStatus.failed);
    expect(state.syncProfile.lastError, 'local mock sync failed');
  });

  test('retries failed sync queue items', () async {
    final store = MemoryVaultStore();
    final failedItem = _syncQueueItem('queue-failed').copyWith(
      status: SyncQueueItemStatus.failed,
      attempts: 1,
      lastError: 'network failed',
    );
    await store.enqueueSyncItem(failedItem);
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: const LocalMockSyncTransport(),
    );
    await state.load();

    await state.retryFailedSyncQueue();

    final queue = await store.loadSyncQueue();
    expect(queue.single.status, SyncQueueItemStatus.synced);
    expect(queue.single.attempts, 2);
    expect(queue.single.lastError, isNull);
    expect(queue.single.nextRetryAt, isNull);
    expect(state.failedSyncQueueCount, 0);
  });

  test('skips failed sync queue items before retry backoff expires', () async {
    final store = MemoryVaultStore();
    final failedItem = _syncQueueItem('queue-backoff').copyWith(
      status: SyncQueueItemStatus.failed,
      attempts: 1,
      lastError: 'network failed',
      nextRetryAt: DateTime.now().add(const Duration(minutes: 5)),
    );
    await store.enqueueSyncItem(failedItem);
    final transport = _RecordingBatchTransport();
    final state = MobileAppState(vaultStore: store, syncTransport: transport);
    await state.load();

    await state.syncPendingQueue();

    final queue = await store.loadSyncQueue();
    expect(transport.batchCalls, 0);
    expect(queue.single.status, SyncQueueItemStatus.failed);
    expect(queue.single.attempts, 1);
  });

  test('manually retries failed queue items even during backoff', () async {
    final store = MemoryVaultStore();
    final failedItem = _syncQueueItem('queue-manual-backoff').copyWith(
      status: SyncQueueItemStatus.failed,
      attempts: 1,
      lastError: 'network failed',
      nextRetryAt: DateTime.now().add(const Duration(hours: 1)),
    );
    await store.enqueueSyncItem(failedItem);
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: const LocalMockSyncTransport(),
    );
    await state.load();

    await state.retryFailedSyncQueue();

    final queue = await store.loadSyncQueue();
    expect(queue.single.status, SyncQueueItemStatus.synced);
    expect(queue.single.attempts, 2);
    expect(queue.single.nextRetryAt, isNull);
  });

  test('stops automatic sync after max failed attempts', () async {
    final store = MemoryVaultStore();
    final failedItem = _syncQueueItem('queue-max-attempts').copyWith(
      status: SyncQueueItemStatus.failed,
      attempts: MobileAppState.syncQueueMaxAttempts,
      lastError: 'network failed',
    );
    await store.enqueueSyncItem(failedItem);
    final transport = _RecordingBatchTransport();
    final state = MobileAppState(vaultStore: store, syncTransport: transport);
    await state.load();

    await state.syncPendingQueue();

    final queue = await store.loadSyncQueue();
    expect(transport.batchCalls, 0);
    expect(queue.single.status, SyncQueueItemStatus.failed);
    expect(queue.single.attempts, MobileAppState.syncQueueMaxAttempts);
  });

  test('saves and loads LAN debug sync profile', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: '123456',
    );

    expect(state.cloudSyncEnabled, isFalse);
    expect(state.syncTransportMode, SyncTransportMode.lanDebug);
    expect(state.syncProfile.status, SyncConnectionStatus.connected);
    expect(state.syncProfile.lanDebugAddress, 'http://127.0.0.1:47219');

    final reloaded = MobileAppState(vaultStore: store);
    await reloaded.load();

    expect(reloaded.cloudSyncEnabled, isFalse);
    expect(reloaded.syncProfile.lanDebugPairingCode, '123456');
  });

  test(
    'continue with account creates or loads account identity contract',
    () async {
      final store = MemoryVaultStore();
      final state = MobileAppState(vaultStore: store);
      await state.load();
      state.updateCreatorLabel('Alice Creator');

      await state.continueWithAccountPlaceholder(
        accountLabel: 'alice@example.com',
        password: 'password-123',
      );

      expect(state.cloudSyncEnabled, isFalse);
      expect(state.canUseCloudSync, isFalse);
      expect(state.syncProfile.accountId, startsWith('acct_'));
      expect(state.syncProfile.workspaceId, startsWith('ws_'));
      expect(state.syncProfile.workspaceName, '个人空间');
      expect(state.syncProfile.deviceId, startsWith('dev_'));
      expect(state.syncProfile.deviceRegistered, isTrue);
      expect(state.syncProfile.creatorProfileId, startsWith('creator_'));
      expect(state.syncProfile.creatorDisplayName, 'Alice Creator');
      expect(state.syncProfile.creatorProfileSynced, isTrue);
      expect(state.syncProfile.entitlementId, startsWith('ent_'));
      expect(state.syncProfile.entitlementPlanCode, 'free');
      expect(state.syncProfile.entitlementFeatures['cloud_sync'], isFalse);

      final reloaded = MobileAppState(vaultStore: store);
      await reloaded.load();

      expect(reloaded.cloudSyncEnabled, isFalse);
      expect(reloaded.syncProfile.accountLabel, 'alice@example.com');
      expect(reloaded.syncProfile.workspaceName, '个人空间');
      expect(reloaded.syncProfile.deviceRegistered, isTrue);
      expect(reloaded.syncProfile.creatorDisplayName, 'Alice Creator');
      expect(reloaded.syncProfile.entitlementFeatures['cloud_sync'], isFalse);
    },
  );

  test('continue with the same account is idempotent', () async {
    final store = MemoryVaultStore();
    final state = MobileAppState(vaultStore: store);
    await state.load();

    await state.continueWithAccountPlaceholder(
      accountLabel: 'alice@example.com',
      password: 'password-123',
    );
    final firstAccountId = state.syncProfile.accountId;
    final firstWorkspaceId = state.syncProfile.workspaceId;
    final firstCreatorId = state.syncProfile.creatorProfileId;

    await state.signOutCloud();
    await state.continueWithAccountPlaceholder(
      accountLabel: 'alice@example.com',
      password: 'password-123',
    );

    expect(state.syncProfile.accountId, firstAccountId);
    expect(state.syncProfile.workspaceId, firstWorkspaceId);
    expect(state.syncProfile.creatorProfileId, firstCreatorId);
  });

  test(
    'sign out keeps local vault records and sync queue for local use',
    () async {
      final store = MemoryVaultStore();
      final state = MobileAppState(vaultStore: store);
      await state.load();
      await state.continueWithAccountPlaceholder(
        accountLabel: 'alice@example.com',
        password: 'password-123',
      );
      state.addWriteResult(
        result: const WatermarkWriteResult(
          kind: WatermarkAssetKind.image,
          bytes: [1, 2, 3],
          watermarkUid: 'uid-retained',
          revision: 1,
          sha256: 'hash-retained',
          seed: _testSeed,
          processTimeMs: 1234,
          verification: WatermarkWriteVerification(
            verified: true,
            watermarkUid: 'uid-retained',
            revision: 1,
            message: '已回读验证版权编号，保护副本可取证。',
          ),
        ),
        fileName: 'retained.png',
        allowRewrite: false,
      );

      await state.signOutCloud();

      expect(state.syncTransportMode, SyncTransportMode.localOnly);
      expect(state.syncProfile.accountId, isNull);
      expect(state.syncProfile.authToken, isNull);
      expect(state.records.single.watermarkUid, 'uid-retained');
      expect(state.syncQueue.single.recordId, state.records.single.id);

      final reloaded = MobileAppState(vaultStore: store);
      await reloaded.load();
      expect(reloaded.records.single.watermarkUid, 'uid-retained');
      expect(reloaded.syncQueue.single.recordId, reloaded.records.single.id);
      expect(reloaded.syncProfile.accountId, isNull);
    },
  );

  test(
    'continue with account can apply cloud auth continue response',
    () async {
      final store = MemoryVaultStore();
      final cloudClient = CloudAccountClient(
        baseUrl: 'https://api.hiddenshield.test',
        client: MockClient((request) async {
          return http.Response.bytes(
            utf8.encode(
              jsonEncode({
                'accessToken': 'access-token',
                'refreshToken': 'refresh-token',
                'account': {
                  'id': 'acct-cloud',
                  'displayName': 'alice@example.com',
                },
                'workspace': {'id': 'ws-cloud', 'name': '个人空间'},
                'device': {'id': 'device-cloud', 'registered': true},
                'creatorProfile': {
                  'id': 'creator-cloud',
                  'displayName': 'Alice Creator',
                  'isDefault': true,
                },
                'entitlement': {
                  'id': 'ent-cloud',
                  'planName': 'Creator',
                  'planCode': 'creator',
                  'status': 'active',
                  'features': {'cloud_sync': true},
                },
              }),
            ),
            200,
            headers: const {'content-type': 'application/json; charset=utf-8'},
          );
        }),
      );
      final state = MobileAppState(
        vaultStore: store,
        cloudAccountClient: cloudClient,
      );
      await state.load();
      state.updateCreatorLabel('Alice Creator');

      await state.continueWithAccountPlaceholder(
        accountLabel: 'alice@example.com',
        password: 'password-123',
      );

      expect(state.syncProfile.accountId, 'acct-cloud');
      expect(state.syncProfile.authToken, 'access-token');
      expect(state.syncProfile.refreshToken, 'refresh-token');
      expect(state.syncProfile.workspaceId, 'ws-cloud');
      expect(state.syncProfile.deviceId, 'device-cloud');
      expect(state.syncProfile.creatorProfileId, 'creator-cloud');
      expect(state.syncProfile.entitlementId, 'ent-cloud');
      expect(state.cloudSyncEnabled, isTrue);
      expect(state.canUseCloudSync, isTrue);
    },
  );

  test(
    'creator cloud sign-in automatically pulls then flushes pending vault queue',
    () async {
      final store = MemoryVaultStore();
      await store.enqueueSyncItem(_syncQueueItem('auto-cloud-queue-1'));
      final transport = _AutoCloudSyncTransport();
      final cloudClient = CloudAccountClient(
        baseUrl: 'https://api.hiddenshield.test',
        client: MockClient((request) async {
          return http.Response.bytes(
            utf8.encode(
              jsonEncode({
                'accessToken': 'access-token',
                'refreshToken': 'refresh-token',
                'account': {
                  'id': 'acct-cloud',
                  'displayName': 'alice@example.com',
                },
                'workspace': {'id': 'ws-cloud', 'name': '个人空间'},
                'device': {'id': 'device-cloud', 'registered': true},
                'creatorProfile': {
                  'id': 'creator-cloud',
                  'displayName': 'Alice Creator',
                  'isDefault': true,
                },
                'entitlement': {
                  'id': 'ent-cloud',
                  'planName': 'Creator',
                  'planCode': 'creator',
                  'status': 'active',
                  'features': {'cloud_sync': true},
                },
                'syncPolicy': 'auto_cloud_vault',
                'cloudVaultCursor': 'cursor-before-login',
              }),
            ),
            200,
            headers: const {'content-type': 'application/json; charset=utf-8'},
          );
        }),
      );
      final state = MobileAppState(
        vaultStore: store,
        cloudAccountClient: cloudClient,
        syncTransport: transport,
      );
      await state.load();

      await state.continueWithAccountPlaceholder(
        accountLabel: 'alice@example.com',
        password: 'password-123',
      );

      expect(state.syncProfile.syncPolicy, 'auto_cloud_vault');
      expect(state.syncProfile.lastRemotePullCursor, 'cursor-after-pull-2');
      expect(transport.fetchCalls, 2);
      expect(transport.batchCalls, 1);
      expect(state.pendingSyncQueueCount, 0);
    },
  );

  test(
    'creator can pause and resume automatic cloud vault sync without clearing queue',
    () async {
      final store = MemoryVaultStore();
      await store.enqueueSyncItem(_syncQueueItem('paused-cloud-queue-1'));
      final transport = _AutoCloudSyncTransport();
      var autoSyncEnabled = true;
      final cloudClient = CloudAccountClient(
        baseUrl: 'https://api.hiddenshield.test',
        client: MockClient((request) async {
          if (request.url.path == '/v1/auth/sessions') {
            return http.Response.bytes(
              utf8.encode(
                jsonEncode({
                  'accessToken': 'access-token',
                  'refreshToken': 'refresh-token',
                  'account': {
                    'id': 'acct-cloud',
                    'displayName': 'alice@example.com',
                  },
                  'workspace': {'id': 'ws-cloud', 'name': '个人空间'},
                  'device': {'id': 'device-cloud', 'registered': true},
                  'creatorProfile': {
                    'id': 'creator-cloud',
                    'displayName': 'Alice Creator',
                    'isDefault': true,
                  },
                  'entitlement': {
                    'id': 'ent-cloud',
                    'planName': 'Creator',
                    'planCode': 'creator',
                    'status': 'active',
                    'features': {'cloud_sync': true},
                  },
                  'syncPolicy': autoSyncEnabled
                      ? 'auto_cloud_vault'
                      : 'manual_local_only',
                  'cloudVaultCursor': 'cursor-before-login',
                }),
              ),
              200,
              headers: const {
                'content-type': 'application/json; charset=utf-8',
              },
            );
          }
          if (request.url.path == '/v1/me/sync-preferences') {
            final body = jsonDecode(request.body) as Map<String, Object?>;
            autoSyncEnabled = body['autoSyncEnabled'] == true;
            return http.Response.bytes(
              utf8.encode(
                jsonEncode({
                  'syncPolicy': autoSyncEnabled
                      ? 'auto_cloud_vault'
                      : 'manual_local_only',
                  'autoSyncEnabled': autoSyncEnabled,
                  'cloudVaultCursor': 'cursor-pref',
                  'entitlement': {
                    'id': 'ent-cloud',
                    'planName': 'Creator',
                    'planCode': 'creator',
                    'status': 'active',
                    'features': {'cloud_sync': true},
                  },
                }),
              ),
              200,
              headers: const {
                'content-type': 'application/json; charset=utf-8',
              },
            );
          }
          return http.Response('not found', 404);
        }),
      );
      final state = MobileAppState(
        vaultStore: store,
        cloudAccountClient: cloudClient,
        syncTransport: transport,
      );
      await state.load();

      await state.continueWithAccountPlaceholder(
        accountLabel: 'alice@example.com',
        password: 'password-123',
      );
      expect(state.syncProfile.syncPolicy, 'auto_cloud_vault');
      expect(transport.fetchCalls, 2);
      expect(transport.batchCalls, 1);

      await state.setAutomaticCloudSyncEnabled(false);
      expect(state.syncProfile.syncPolicy, 'manual_local_only');
      expect(state.cloudSyncEnabled, isFalse);
      expect(state.canUseCloudSync, isTrue);
      expect(state.canAutoCloudSync, isFalse);
      expect(transport.fetchCalls, 2);
      expect(transport.batchCalls, 1);

      await state.setAutomaticCloudSyncEnabled(true);
      expect(state.syncProfile.syncPolicy, 'auto_cloud_vault');
      expect(state.cloudSyncEnabled, isTrue);
      expect(state.canAutoCloudSync, isTrue);
      expect(transport.fetchCalls, 4);
      expect(transport.batchCalls, 1);
    },
  );

  test(
    'requesting watermark reissue marks mobile record pending repair and syncs it',
    () async {
      final store = MemoryVaultStore();
      Map<String, Object?>? reissueRequestBody;
      final cloudClient = CloudAccountClient(
        baseUrl: 'https://api.hiddenshield.test',
        client: MockClient((request) async {
          if (request.url.path == '/v1/auth/sessions') {
            return http.Response.bytes(
              utf8.encode(
                jsonEncode({
                  'accessToken': 'access-token',
                  'refreshToken': 'refresh-token',
                  'account': {
                    'id': 'acct-cloud',
                    'displayName': 'alice@example.com',
                  },
                  'workspace': {'id': 'ws-cloud', 'name': '个人空间'},
                  'device': {'id': 'device-cloud', 'registered': true},
                  'creatorProfile': {
                    'id': 'creator-cloud',
                    'displayName': 'Alice Creator',
                    'isDefault': true,
                  },
                  'entitlement': {
                    'id': 'ent-cloud',
                    'planName': 'Creator',
                    'planCode': 'creator',
                    'status': 'active',
                    'features': {'cloud_sync': true},
                  },
                }),
              ),
              200,
              headers: const {
                'content-type': 'application/json; charset=utf-8',
              },
            );
          }
          if (request.url.path == '/v1/watermark-ids/reissue') {
            expect(request.headers['authorization'], 'Bearer access-token');
            reissueRequestBody =
                jsonDecode(request.body) as Map<String, Object?>;
            return http.Response.bytes(
              utf8.encode(
                jsonEncode({
                  'jobId': 'reissue-job-1',
                  'previousWatermarkUid':
                      'HS-11111111-22222222-33333333-44444444',
                  'replacement': {
                    'registryId': 'registry-reissue-1',
                    'watermarkUid': 'HS-55555555-66666666-77777777-88888888',
                    'watermarkIdIssueMode': 'server_reissued',
                    'registryStatus': 'server_confirmed',
                    'registryReceipt': 'receipt-reissue-1',
                    'registryProofHash': 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
                    'payloadProtocolVersion': 2,
                    'payloadBytesLength': 119,
                    'parentWatermarkUid':
                        'HS-11111111-22222222-33333333-44444444',
                    'revision': 2,
                    'issuedAt': '2026-06-27T00:00:01Z',
                    'updatedAt': '2026-06-27T00:00:01Z',
                  },
                }),
              ),
              200,
              headers: const {
                'content-type': 'application/json; charset=utf-8',
              },
            );
          }
          return http.Response('not found', 404);
        }),
      );
      final state = MobileAppState(
        vaultStore: store,
        cloudAccountClient: cloudClient,
      );
      await state.load();
      state.updateCreatorLabel('Alice Creator');
      await state.continueWithAccountPlaceholder(
        accountLabel: 'alice@example.com',
        password: 'password-123',
      );
      final record = state.addWriteResult(
        result: const WatermarkWriteResult(
          kind: WatermarkAssetKind.image,
          bytes: [1, 2, 3],
          watermarkUid: 'HS-11111111-22222222-33333333-44444444',
          revision: 1,
          sha256:
              'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          seed: _testSeed,
          processTimeMs: 1234,
          verification: WatermarkWriteVerification(
            verified: true,
            watermarkUid: 'HS-11111111-22222222-33333333-44444444',
            revision: 1,
            message: '已回读验证版权编号，保护副本可取证。',
          ),
        ),
        fileName: 'duplicate.png',
        allowRewrite: false,
      );

      final message = await state.requestWatermarkReissueForRecord(record);

      expect(message, contains('已创建重新签发任务'));
      expect(reissueRequestBody?['workspaceId'], 'ws-cloud');
      expect(reissueRequestBody?['creatorProfileId'], 'creator-cloud');
      expect(
        reissueRequestBody?['previousWatermarkUid'],
        'HS-11111111-22222222-33333333-44444444',
      );
      expect(
        reissueRequestBody?['parentWatermarkUid'],
        'HS-11111111-22222222-33333333-44444444',
      );
      expect(reissueRequestBody?['revision'], 2);
      expect(
        reissueRequestBody?['originalHash'],
        'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      );

      final updated = state.records.singleWhere((item) => item.id == record.id);
      expect(updated.watermarkIdRegistryStatus, 'reissue_required');
      expect(updated.watermarkIdRegistryReceipt, 'receipt-reissue-1');
      expect(updated.payloadAuthStatus, 'pending_repair');
      expect(updated.syncStatus, SyncStatus.conflict);
      expect(updated.writeVerificationStatus, WriteVerificationStatus.failed);
      expect(updated.writeVerificationMessage, contains('reissue-job-1'));
      expect(
        updated.writeVerificationMessage,
        contains('HS-55555555-66666666-77777777-88888888'),
      );

      final persisted = await store.loadRecords();
      expect(persisted.single.watermarkIdRegistryStatus, 'reissue_required');
      expect(persisted.single.payloadAuthStatus, 'pending_repair');

      final queue = await store.loadSyncQueue();
      expect(queue, hasLength(1));
      final payload =
          jsonDecode(queue.single.payloadJson) as Map<String, Object?>;
      expect(payload['watermark_id_registry_status'], 'reissue_required');
      expect(payload['watermark_id_registry_receipt'], 'receipt-reissue-1');
      expect(payload['payload_auth_status'], 'pending_repair');
      expect(payload.keys, everyElement(isIn(vaultRecordSyncPayloadKeys)));
    },
  );

  test(
    'continue with account hides technical network errors from user-facing state',
    () async {
      final store = MemoryVaultStore();
      final cloudClient = CloudAccountClient(
        baseUrl: 'http://127.0.0.1:43188',
        client: MockClient((request) async {
          throw http.ClientException(
            'Failed to fetch',
            Uri.parse('http://127.0.0.1:43188/v1/auth/sessions'),
          );
        }),
      );
      final state = MobileAppState(
        vaultStore: store,
        cloudAccountClient: cloudClient,
      );
      await state.load();
      state.updateCreatorLabel('Alice Creator');

      final signedIn = await state.continueWithAccountPlaceholder(
        accountLabel: 'alice@example.com',
        password: 'password-123',
      );

      expect(signedIn, isFalse);
      expect(state.syncProfile.lastError, contains('暂时无法连接服务'));
      expect(state.syncProfile.lastError, isNot(contains('ClientException')));
      expect(state.syncProfile.lastError, isNot(contains('Failed to fetch')));
      expect(state.syncProfile.lastError, isNot(contains('/v1/auth/sessions')));
    },
  );

  test('test desktop connection returns paired status', () async {
    final state = MobileAppState(vaultStore: MemoryVaultStore());
    await state.load();
    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.testLanDebugConnection();

    expect(state.syncProfile.status, SyncConnectionStatus.connected);
    expect(state.syncProfile.lastError, isNull);
  });

  test('enables LAN debug sync mode only after debug pairing', () async {
    final state = MobileAppState(vaultStore: MemoryVaultStore());
    await state.load();

    state.setSyncTransportMode(SyncTransportMode.lanDebug);
    expect(state.syncTransportMode, SyncTransportMode.localOnly);

    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );
    state.setSyncTransportMode(SyncTransportMode.lanDebug);

    expect(state.syncTransportMode, SyncTransportMode.lanDebug);
  });

  test('uses selected transport mode when syncing', () async {
    final usedModes = <SyncTransportMode>[];
    final state = MobileAppState(
      vaultStore: MemoryVaultStore(),
      syncTransportFactory: (mode, syncProfile) {
        usedModes.add(mode);
        return const LocalMockSyncTransport();
      },
    );
    await state.load();
    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );
    state.setSyncTransportMode(SyncTransportMode.lanDebug);

    state.addWriteResult(
      result: const WatermarkWriteResult(
        kind: WatermarkAssetKind.image,
        bytes: [1, 2, 3],
        watermarkUid: 'uid-http',
        revision: 1,
        sha256: 'hash',
        seed: _testSeed,
        processTimeMs: 1234,
        verification: WatermarkWriteVerification(
          verified: true,
          watermarkUid: 'verified-uid',
          revision: 1,
          message: '已回读验证版权编号，保护副本可取证。',
        ),
      ),
      fileName: 'http.png',
      allowRewrite: false,
    );

    await state.syncPendingQueue();

    expect(usedModes, contains(SyncTransportMode.lanDebug));
    expect(state.pendingSyncQueueCount, 0);
  });

  test('syncs multiple pending items in one batch transport call', () async {
    final store = MemoryVaultStore();
    await store.enqueueSyncItem(_syncQueueItem('queue-batch-1'));
    await store.enqueueSyncItem(_syncQueueItem('queue-batch-2'));

    final transport = _RecordingBatchTransport();
    final state = MobileAppState(vaultStore: store, syncTransport: transport);
    await state.load();

    await state.syncPendingQueue();

    expect(transport.batchCalls, 1);
    expect(transport.batchSizes.single, 2);
    expect(state.pendingSyncQueueCount, 0);
  });

  test('pulls desktop changes into the local vault', () async {
    final store = MemoryVaultStore();
    final transport = _DesktopChangesTransport();
    final state = MobileAppState(vaultStore: store, syncTransport: transport);
    await state.load();
    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.pullRemoteChanges();

    final records = await store.loadRecords();
    expect(transport.fetchCalls, 1);
    expect(records, hasLength(2));
    final desktopRecord = records.firstWhere(
      (record) => record.watermarkUid == 'uid-desktop',
    );
    final evidenceRecord = records.firstWhere(
      (record) => record.watermarkUid == 'uid-evidence',
    );
    expect(desktopRecord.id, 'lan:desktop-1');
    expect(desktopRecord.syncStatus, SyncStatus.synced);
    expect(evidenceRecord.source, VaultRecordSource.verify);
    expect(evidenceRecord.extractedTimestamp, 123);
    expect(evidenceRecord.extractedDeviceIdHex, 'device');
    expect(evidenceRecord.extractedFileHashHex, 'hash');
    expect(state.syncProfile.lastError, isNull);
    expect(state.syncProfile.lastRemotePullCursor, '2026-06-16T12:00:00.000Z');
    expect(state.syncProfile.lastSyncAttemptAt, isNotNull);
    expect(state.syncProfile.lastSyncSuccessAt, isNotNull);
    expect(state.syncProfile.lastSyncFailureAt, isNull);

    final reloaded = MobileAppState(
      vaultStore: store,
      syncTransport: transport,
    );
    await reloaded.load();
    expect(
      reloaded.syncProfile.lastRemotePullCursor,
      '2026-06-16T12:00:00.000Z',
    );

    await reloaded.pullRemoteChanges();
    expect(transport.lastSince, '2026-06-16T12:00:00.000Z');
  });

  test(
    'deduplicates desktop pull records with the same uid hash and revision',
    () async {
      final store = MemoryVaultStore();
      await store.upsertRecord(
        _vaultRecord(
          id: 'local-existing',
          watermarkUid: 'uid-dup',
          revision: 2,
          sha256: 'hash-dup',
        ),
      );
      final state = MobileAppState(
        vaultStore: store,
        syncTransport: _StaticChangesTransport(
          changes: [
            _desktopChange(
              id: 'desktop-duplicate',
              watermarkUid: 'uid-dup',
              revision: 2,
              sha256: 'hash-dup',
            ),
          ],
        ),
      );
      await state.load();
      await state.saveLanDebugPairing(
        lanDebugAddress: 'http://127.0.0.1:47219',
        pairingCode: 'abcdef',
      );

      await state.pullRemoteChanges();

      final records = await store.loadRecords();
      final resolutions = await store.loadSyncResolutions();
      expect(records, hasLength(1));
      expect(records.single.id, 'local-existing');
      expect(
        resolutions.single.resolutionType,
        MobileSyncResolutionType.duplicateIgnored,
      );
      expect(resolutions.single.existingRecordId, 'local-existing');
    },
  );

  test('upgrades same uid and hash to the highest pulled revision', () async {
    final store = MemoryVaultStore();
    await store.upsertRecord(
      _vaultRecord(
        id: 'local-existing',
        title: 'old.png',
        watermarkUid: 'uid-upgrade',
        revision: 1,
        sha256: 'hash-upgrade',
      ),
    );
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: _StaticChangesTransport(
        changes: [
          _desktopChange(
            id: 'desktop-upgrade',
            title: 'new.png',
            watermarkUid: 'uid-upgrade',
            revision: 3,
            sha256: 'hash-upgrade',
          ),
        ],
      ),
    );
    await state.load();
    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.pullRemoteChanges();

    final records = await store.loadRecords();
    final resolutions = await store.loadSyncResolutions();
    expect(records, hasLength(1));
    expect(records.single.id, 'local-existing');
    expect(records.single.title, 'new.png');
    expect(records.single.revision, 3);
    expect(
      resolutions.single.resolutionType,
      MobileSyncResolutionType.revisionUpgraded,
    );
  });

  test('ignores stale desktop revisions for the same uid and hash', () async {
    final store = MemoryVaultStore();
    await store.upsertRecord(
      _vaultRecord(
        id: 'local-existing',
        watermarkUid: 'uid-stale',
        revision: 5,
        sha256: 'hash-stale',
      ),
    );
    final state = MobileAppState(
      vaultStore: store,
      syncTransport: _StaticChangesTransport(
        changes: [
          _desktopChange(
            id: 'desktop-stale',
            watermarkUid: 'uid-stale',
            revision: 3,
            sha256: 'hash-stale',
          ),
        ],
      ),
    );
    await state.load();
    await state.saveLanDebugPairing(
      lanDebugAddress: 'http://127.0.0.1:47219',
      pairingCode: 'abcdef',
    );

    await state.pullRemoteChanges();

    final records = await store.loadRecords();
    final resolutions = await store.loadSyncResolutions();
    expect(records, hasLength(1));
    expect(records.single.revision, 5);
    expect(
      resolutions.single.resolutionType,
      MobileSyncResolutionType.staleRevisionIgnored,
    );
  });

  test(
    'keeps same uid different hash records while awaiting registry arbitration',
    () async {
      final store = MemoryVaultStore();
      await store.upsertRecord(
        _vaultRecord(
          id: 'local-existing',
          watermarkUid: 'uid-variant',
          revision: 2,
          sha256: 'hash-a',
        ),
      );
      final state = MobileAppState(
        vaultStore: store,
        syncTransport: _StaticChangesTransport(
          changes: [
            _desktopChange(
              id: 'desktop-variant',
              watermarkUid: 'uid-variant',
              revision: 2,
              sha256: 'hash-b',
            ),
          ],
        ),
      );
      await state.load();
      await state.saveLanDebugPairing(
        lanDebugAddress: 'http://127.0.0.1:47219',
        pairingCode: 'abcdef',
      );

      await state.pullRemoteChanges();

      final records = await store.loadRecords();
      final resolutions = await store.loadSyncResolutions();
      expect(records, hasLength(2));
      expect(
        records.map((record) => record.sha256),
        containsAll(['hash-a', 'hash-b']),
      );
      expect(
        resolutions.single.resolutionType,
        MobileSyncResolutionType.pendingRegistryReconcile,
      );
      expect(resolutions.single.insertedRecordId, 'lan:desktop-variant');
      expect(
        records
            .firstWhere((record) => record.sha256 == 'hash-b')
            .watermarkIdRegistryStatus,
        'pending_registry_reconcile',
      );
    },
  );

  test(
    'does not overwrite same id local record while awaiting registry arbitration',
    () async {
      final store = MemoryVaultStore();
      await store.upsertRecord(
        _vaultRecord(
          id: 'shared-id',
          watermarkUid: 'uid-shared',
          revision: 2,
          sha256: 'hash-local',
          title: 'local.png',
        ),
      );
      final state = MobileAppState(
        vaultStore: store,
        syncTransport: _StaticChangesTransport(
          changes: [
            _desktopChange(
              id: 'shared-id',
              title: 'remote.png',
              watermarkUid: 'uid-shared',
              revision: 2,
              sha256: 'hash-remote',
            ),
          ],
        ),
      );
      await state.load();
      await state.saveLanDebugPairing(
        lanDebugAddress: 'http://127.0.0.1:47219',
        pairingCode: 'abcdef',
      );

      await state.pullRemoteChanges();

      final records = await store.loadRecords();
      final resolutions = await store.loadSyncResolutions();
      expect(records, hasLength(2));
      expect(
        records.map((record) => record.sha256),
        containsAll(['hash-local', 'hash-remote']),
      );
      expect(
        resolutions.single.resolutionType,
        MobileSyncResolutionType.pendingRegistryReconcile,
      );
      expect(resolutions.single.existingRecordId, 'shared-id');
      expect(
        records
            .firstWhere((record) => record.sha256 == 'hash-remote')
            .watermarkIdRegistryStatus,
        'pending_registry_reconcile',
      );
    },
  );
}

VaultRecord _vaultRecord({
  required String id,
  required String watermarkUid,
  required int revision,
  required String sha256,
  String title = 'work.png',
}) {
  return VaultRecord(
    id: id,
    kind: WatermarkAssetKind.image,
    title: title,
    watermarkUid: watermarkUid,
    revision: revision,
    sha256: sha256,
    writeVerificationStatus: WriteVerificationStatus.verified,
    writeVerificationMessage: '完成后验证已通过',
    writeVerificationAt: DateTime.fromMillisecondsSinceEpoch(2000),
    source: VaultRecordSource.write,
    syncStatus: SyncStatus.synced,
    createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
  );
}

RemoteSyncChange _desktopChange({
  required String id,
  required String watermarkUid,
  required int revision,
  required String sha256,
  String title = 'desktop.png',
  String? writeVerificationStatus,
  String? writeVerificationMessage,
  String? writeVerificationAt,
}) {
  return RemoteSyncChange(
    id: id,
    kind: 'image',
    title: title,
    watermarkUid: watermarkUid,
    revision: revision,
    sha256: sha256,
    writeVerificationStatus: writeVerificationStatus,
    writeVerificationMessage: writeVerificationMessage,
    writeVerificationAt: writeVerificationAt,
    sourceDevice: 'lanDebug',
    createdAt: '2026-06-16T12:00:00.000Z',
  );
}

SyncQueueItem _syncQueueItem(String id) {
  return SyncQueueItem(
    id: id,
    recordId: '$id-record',
    operation: SyncQueueOperation.upsertVaultRecord,
    payloadType: 'vault_record',
    payloadJson: '{"id":"$id-record","kind":"image","watermark_uid":"$id-uid"}',
    status: SyncQueueItemStatus.pending,
    attempts: 0,
    createdAt: DateTime.fromMillisecondsSinceEpoch(1000),
  );
}

class _StaticChangesTransport implements SyncTransport {
  const _StaticChangesTransport({required this.changes});

  final List<RemoteSyncChange> changes;

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    return const SyncSendResult.success();
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    return SyncBatchSendResult({
      for (final item in items) item.id: const SyncSendResult.success(),
    });
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    return SyncChangesResult.success(
      changes: changes,
      nextSince: '2026-06-16T12:00:00.000Z',
    );
  }
}

class _RecordingBatchTransport implements SyncTransport {
  int batchCalls = 0;
  final List<int> batchSizes = [];

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    throw StateError('single-item send should not be used');
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    batchCalls += 1;
    batchSizes.add(items.length);
    return SyncBatchSendResult({
      for (final item in items) item.id: const SyncSendResult.success(),
    });
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    return const SyncChangesResult.success(changes: [], nextSince: '');
  }
}

class _AutoCloudSyncTransport implements SyncTransport {
  int fetchCalls = 0;
  int batchCalls = 0;

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    return const SyncSendResult.success();
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    batchCalls += 1;
    return SyncBatchSendResult({
      for (final item in items) item.id: const SyncSendResult.success(),
    });
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    fetchCalls += 1;
    return SyncChangesResult.success(
      changes: const [],
      nextSince: 'cursor-after-pull-$fetchCalls',
    );
  }
}

class _DesktopChangesTransport implements SyncTransport {
  int fetchCalls = 0;
  String? lastSince;

  @override
  Future<SyncSendResult> send(SyncQueueItem item) async {
    return const SyncSendResult.success();
  }

  @override
  Future<SyncBatchSendResult> sendBatch(List<SyncQueueItem> items) async {
    return SyncBatchSendResult({
      for (final item in items) item.id: const SyncSendResult.success(),
    });
  }

  @override
  Future<SyncChangesResult> fetchChanges({String? since}) async {
    fetchCalls += 1;
    lastSince = since;
    return const SyncChangesResult.success(
      nextSince: '2026-06-16T12:00:00.000Z',
      changes: [
        RemoteSyncChange(
          id: 'desktop-1',
          kind: 'image',
          title: 'desktop.png',
          watermarkUid: 'uid-desktop',
          revision: 2,
          sha256: 'hash-desktop',
          writeVerificationStatus: 'verified',
          writeVerificationMessage: '完成后验证已通过',
          writeVerificationAt: '2026-06-16T12:00:01.000Z',
          sourceDevice: 'lanDebug',
          createdAt: '2026-06-16T12:00:00.000Z',
        ),
        RemoteSyncChange(
          id: 'desktop-evidence-1',
          kind: 'audio',
          title: 'suspect.wav',
          watermarkUid: 'uid-evidence',
          revision: 3,
          source: 'verify',
          extractedTimestamp: 123,
          extractedDeviceIdHex: 'device',
          extractedFileHashHex: 'hash',
          sourceDevice: 'lanDebug',
          createdAt: '2026-06-16T12:00:01.000Z',
        ),
      ],
    );
  }
}
