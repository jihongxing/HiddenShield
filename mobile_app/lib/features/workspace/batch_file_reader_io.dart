import 'dart:io';
import 'dart:typed_data';

Future<Uint8List> readBatchFileBytes(String path) => File(path).readAsBytes();
