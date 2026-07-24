import 'dart:convert';

import 'package:http/http.dart' as http;

import 'system_config.dart';

class MobileAnonymousFeedbackClient {
  MobileAnonymousFeedbackClient({http.Client? httpClient, Uri? endpoint})
    : _httpClient = httpClient ?? http.Client(),
      _endpoint =
          endpoint ??
          Uri.parse(
            '${HiddenShieldSystemConfig.fallback.cloudBaseUrl}/v1/anonymous-feedback/batches',
          );

  final http.Client _httpClient;
  final Uri _endpoint;

  Uri get endpoint => _endpoint;

  bool get endpointConfigured => _endpoint.toString().trim().isNotEmpty;

  Future<MobileAnonymousFeedbackAck> sendBatch(
    MobileAnonymousFeedbackBatch batch,
  ) async {
    final response = await _httpClient
        .post(
          _endpoint,
          headers: const {'content-type': 'application/json'},
          body: jsonEncode(batch.toJson()),
        )
        .timeout(const Duration(seconds: 6));
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw StateError('HTTP ${response.statusCode}');
    }
    final json = jsonDecode(response.body) as Map<String, Object?>;
    return MobileAnonymousFeedbackAck(
      receivedEvents: (json['receivedEvents'] as num?)?.toInt() ?? 0,
      insertedEvents: (json['insertedEvents'] as num?)?.toInt() ?? 0,
      duplicateEvents: (json['duplicateEvents'] as num?)?.toInt() ?? 0,
      acceptedAt: DateTime.tryParse(json['acceptedAt'] as String? ?? ''),
    );
  }
}

class MobileAnonymousFeedbackBatch {
  const MobileAnonymousFeedbackBatch({
    required this.installId,
    required this.sessionId,
    required this.appVersion,
    required this.sentAt,
    required this.events,
  });

  final String installId;
  final String sessionId;
  final String appVersion;
  final DateTime sentAt;
  final List<MobileAnonymousFeedbackEvent> events;

  Map<String, Object?> toJson() {
    return {
      'installId': installId,
      'sessionId': sessionId,
      'appVersion': appVersion,
      'sentAt': sentAt.toUtc().toIso8601String(),
      'events': events.map((event) => event.toJson()).toList(),
    };
  }
}

class MobileAnonymousFeedbackEvent {
  const MobileAnonymousFeedbackEvent({
    required this.eventId,
    required this.occurredAt,
    required this.installId,
    required this.sessionId,
    required this.appVersion,
    required this.featureName,
    required this.outcome,
    required this.mediaType,
    required this.fileSizeBucket,
    this.durationMs,
    this.errorCode,
    this.diagnosticNote,
    this.stackSummary,
    this.pipelineId,
  });

  final String eventId;
  final DateTime occurredAt;
  final String installId;
  final String sessionId;
  final String appVersion;
  final String featureName;
  final String outcome;
  final String mediaType;
  final String fileSizeBucket;
  final int? durationMs;
  final String? errorCode;
  final String? diagnosticNote;
  final String? stackSummary;
  final String? pipelineId;

  Map<String, Object?> toJson() {
    return {
      'eventId': eventId,
      'occurredAt': occurredAt.toUtc().toIso8601String(),
      'installId': installId,
      'sessionId': sessionId,
      'appVersion': appVersion,
      'featureName': featureName,
      'outcome': outcome,
      'mediaType': mediaType,
      'fileSizeBucket': fileSizeBucket,
      'durationMs': durationMs,
      'errorCode': errorCode,
      'diagnosticNote': diagnosticNote,
      'stackSummary': stackSummary,
      'pipelineId': pipelineId,
    };
  }

  factory MobileAnonymousFeedbackEvent.fromJson(Map<String, Object?> json) {
    return MobileAnonymousFeedbackEvent(
      eventId: json['eventId'] as String? ?? '',
      occurredAt:
          DateTime.tryParse(json['occurredAt'] as String? ?? '') ??
          DateTime.now(),
      installId: json['installId'] as String? ?? '',
      sessionId: json['sessionId'] as String? ?? '',
      appVersion: json['appVersion'] as String? ?? 'mobile',
      featureName: json['featureName'] as String? ?? 'settings_diagnostic',
      outcome: json['outcome'] as String? ?? 'diagnostic',
      mediaType: json['mediaType'] as String? ?? 'none',
      fileSizeBucket: json['fileSizeBucket'] as String? ?? '0-10mb',
      durationMs: (json['durationMs'] as num?)?.toInt(),
      errorCode: json['errorCode'] as String?,
      diagnosticNote: json['diagnosticNote'] as String?,
      stackSummary: json['stackSummary'] as String?,
      pipelineId: json['pipelineId'] as String?,
    );
  }
}

class MobileAnonymousFeedbackAck {
  const MobileAnonymousFeedbackAck({
    required this.receivedEvents,
    required this.insertedEvents,
    required this.duplicateEvents,
    required this.acceptedAt,
  });

  final int receivedEvents;
  final int insertedEvents;
  final int duplicateEvents;
  final DateTime? acceptedAt;
}
