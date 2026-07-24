import 'dart:typed_data';

Future<Uint8List> readReportBundleFileBytes(
  String reportDir,
  String relativePath,
) {
  throw UnsupportedError('当前预览环境不能读取报告包：$reportDir/$relativePath');
}
