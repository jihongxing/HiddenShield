import 'dart:io';
import 'dart:typed_data';

Future<Uint8List> readReportBundleFileBytes(
  String reportDir,
  String relativePath,
) {
  return File('$reportDir${Platform.pathSeparator}$relativePath').readAsBytes();
}
