import '../../app/mobile_app_state.dart';
import '../../bridge/watermark_models.dart';

WatermarkPayloadSeed buildPayloadSeed(
  List<int> bytes,
  MobileAppState appState,
) {
  final creatorIdentity = appState.creatorLabel.trim().isEmpty
      ? '本机创作者'
      : appState.creatorLabel.trim();
  final deviceIdentity =
      appState.syncProfile.deviceId?.trim().isNotEmpty == true
      ? appState.syncProfile.deviceId!.trim()
      : 'mobile-local-device';
  return WatermarkPayloadSeed(
    creatorIdentity: creatorIdentity,
    deviceIdentity: deviceIdentity,
    mediaBytes: bytes,
    timestamp: DateTime.now().millisecondsSinceEpoch ~/ 1000,
  );
}
