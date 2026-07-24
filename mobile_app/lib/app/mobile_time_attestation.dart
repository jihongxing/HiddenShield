import 'dart:convert';

import 'package:http/http.dart' as http;

import 'system_config.dart';

class MobileTrustedTimeAttestation {
  const MobileTrustedTimeAttestation({
    required this.trustedTimeStatus,
    required this.trustedTimeSource,
    required this.trustedTimeAt,
    required this.thirdPartyVerificationStatus,
    required this.thirdPartyVerificationProvider,
    required this.thirdPartyVerificationPath,
  });

  final String trustedTimeStatus;
  final String trustedTimeSource;
  final DateTime trustedTimeAt;
  final String thirdPartyVerificationStatus;
  final String thirdPartyVerificationProvider;
  final String thirdPartyVerificationPath;
}

class MobileTrustedTimeClient {
  MobileTrustedTimeClient({
    http.Client? httpClient,
    Uri? backendEndpoint,
    List<Uri>? endpoints,
  }) : _httpClient = httpClient ?? http.Client(),
       _backendEndpoint =
           backendEndpoint ??
           Uri.parse(
             '${HiddenShieldSystemConfig.fallback.cloudBaseUrl}/v1/trusted-time',
           ),
       _endpoints = endpoints ?? _defaultEndpoints;

  static final List<Uri> _defaultEndpoints = [
    Uri.parse('https://www.aliyun.com'),
    Uri.parse('https://cloud.tencent.com'),
    Uri.parse('https://www.baidu.com'),
  ];

  final http.Client _httpClient;
  final Uri _backendEndpoint;
  final List<Uri> _endpoints;

  Future<MobileTrustedTimeAttestation?> request() async {
    final backendAttestation = await _requestBackendTrustedTime();
    if (backendAttestation != null) {
      return backendAttestation;
    }
    for (final endpoint in _endpoints) {
      try {
        final response = await _httpClient
            .head(endpoint)
            .timeout(const Duration(seconds: 5));
        final dateHeader = response.headers['date'] ?? response.headers['Date'];
        final trustedTimeAt = parseHttpDateHeader(dateHeader);
        if (trustedTimeAt == null) {
          continue;
        }
        return MobileTrustedTimeAttestation(
          trustedTimeStatus: '已记录网络授时',
          trustedTimeSource: endpoint.toString(),
          trustedTimeAt: trustedTimeAt.toUtc(),
          thirdPartyVerificationStatus: '已记录网络授时',
          thirdPartyVerificationProvider: endpoint.host,
          thirdPartyVerificationPath: 'HTTP Date 响应头',
        );
      } catch (_) {
        continue;
      }
    }
    return null;
  }

  Future<MobileTrustedTimeAttestation?> _requestBackendTrustedTime() async {
    try {
      final response = await _httpClient
          .get(_backendEndpoint)
          .timeout(const Duration(seconds: 5));
      if (response.statusCode < 200 || response.statusCode >= 300) {
        return null;
      }
      final json = jsonDecode(response.body) as Map<String, Object?>;
      final trustedTimeAt = DateTime.tryParse(
        json['trustedTimeAt'] as String? ?? '',
      );
      if (trustedTimeAt == null) {
        return null;
      }
      return MobileTrustedTimeAttestation(
        trustedTimeStatus:
            json['status'] as String? ??
            json['trustedTimeStatus'] as String? ??
            '已记录网络授时',
        trustedTimeSource:
            json['source'] as String? ?? _backendEndpoint.toString(),
        trustedTimeAt: trustedTimeAt.toUtc(),
        thirdPartyVerificationStatus:
            json['thirdPartyVerificationStatus'] as String? ?? '已记录网络授时',
        thirdPartyVerificationProvider:
            json['thirdPartyVerificationProvider'] as String? ??
            _backendEndpoint.host,
        thirdPartyVerificationPath:
            json['verificationPath'] as String? ?? 'HiddenShield 后端 HTTP Date',
      );
    } catch (_) {
      return null;
    }
  }
}

DateTime? parseHttpDateHeader(String? value) {
  final trimmed = value?.trim();
  if (trimmed == null || trimmed.isEmpty) {
    return null;
  }
  final iso = DateTime.tryParse(trimmed);
  if (iso != null) {
    return iso.toUtc();
  }
  final match = RegExp(
    r'^(?:[A-Za-z]{3},\s*)?(\d{1,2})\s+([A-Za-z]{3})\s+(\d{4})\s+'
    r'(\d{2}):(\d{2}):(\d{2})\s+GMT$',
    caseSensitive: false,
  ).firstMatch(trimmed);
  if (match == null) {
    return null;
  }
  final month = const {
    'jan': 1,
    'feb': 2,
    'mar': 3,
    'apr': 4,
    'may': 5,
    'jun': 6,
    'jul': 7,
    'aug': 8,
    'sep': 9,
    'oct': 10,
    'nov': 11,
    'dec': 12,
  }[match.group(2)!.toLowerCase()];
  if (month == null) {
    return null;
  }
  return DateTime.utc(
    int.parse(match.group(3)!),
    month,
    int.parse(match.group(1)!),
    int.parse(match.group(4)!),
    int.parse(match.group(5)!),
    int.parse(match.group(6)!),
  );
}
