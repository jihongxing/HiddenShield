import 'dart:convert';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:share_plus/share_plus.dart';

import '../app/mobile_app_state.dart';
import '../shared/theme/design_tokens.dart';
import '../shared/widgets/tool_cards.dart';
import 'offline_license_state.dart';

class OfflineLicensePanel extends StatefulWidget {
  const OfflineLicensePanel({super.key, required this.appState});

  final MobileAppState appState;

  @override
  State<OfflineLicensePanel> createState() => _OfflineLicensePanelState();
}

class _OfflineLicensePanelState extends State<OfflineLicensePanel> {
  bool _busy = false;

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.appState.offlineLicenseSnapshot;
    return HsPanel(
      title: '离线许可证',
      icon: Icons.key_outlined,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _LicenseStatus(snapshot: snapshot),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              FilledButton.icon(
                onPressed: _busy ? null : _createRequest,
                icon: const Icon(Icons.qr_code_2_outlined),
                label: const Text('创建激活请求'),
              ),
              OutlinedButton.icon(
                onPressed: _busy ? null : _importFile,
                icon: const Icon(Icons.file_open_outlined),
                label: const Text('导入文件'),
              ),
              OutlinedButton.icon(
                onPressed: _busy ? null : _pasteToken,
                icon: const Icon(Icons.content_paste_outlined),
                label: const Text('粘贴载荷'),
              ),
              OutlinedButton.icon(
                onPressed: _busy ? null : _scanQr,
                icon: const Icon(Icons.qr_code_scanner_outlined),
                label: const Text('扫描二维码'),
              ),
              if (snapshot.licenseId != null)
                TextButton.icon(
                  onPressed: _busy ? null : _clearLicense,
                  icon: const Icon(Icons.delete_outline),
                  label: const Text('清除许可证'),
                ),
            ],
          ),
          const SizedBox(height: 10),
          Text(
            '只接受 HSLIC1 许可证或 HSRVL1 撤销列表。许可证、安装秘密和盐保存在 '
            'Android Keystore / iOS Keychain；账户同步资料不保存这些值。',
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: HsColors.textMuted),
          ),
        ],
      ),
    );
  }

  Future<void> _createRequest() async {
    await _run(() async {
      final token = await widget.appState.createOfflineActivationRequest();
      if (!mounted) return;
      await showDialog<void>(
        context: context,
        builder: (context) => _ActivationRequestDialog(token: token),
      );
    });
  }

  Future<void> _importFile() async {
    final result = await FilePicker.pickFiles(
      type: FileType.custom,
      allowedExtensions: const ['hslicense', 'hsrvl', 'txt'],
      withData: true,
    );
    final file = result?.files.single;
    if (file == null) return;
    final bytes = file.bytes;
    if (bytes == null) {
      _showMessage('无法读取所选文件。');
      return;
    }
    await _importRaw(utf8.decode(bytes).trim());
  }

  Future<void> _pasteToken() async {
    final controller = TextEditingController();
    final token = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('粘贴离线载荷'),
        content: TextField(
          controller: controller,
          autofocus: true,
          minLines: 4,
          maxLines: 8,
          decoration: const InputDecoration(hintText: 'HSLIC1.… 或 HSRVL1.…'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, controller.text),
            child: const Text('导入'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (token == null) return;
    await _importRaw(token.trim());
  }

  Future<void> _scanQr() async {
    final token = await Navigator.of(context).push<String>(
      MaterialPageRoute(builder: (_) => const _OfflineLicenseScannerPage()),
    );
    if (token == null) return;
    await _importRaw(token);
  }

  Future<void> _importRaw(String token) async {
    await _run(() async {
      if (token.startsWith('HSLIC1.')) {
        await widget.appState.importOfflineLicenseToken(token);
        _showMessage('离线许可证已激活。');
        return;
      }
      if (token.startsWith('HSRVL1.')) {
        await widget.appState.importOfflineRevocationList(token);
        _showMessage('撤销列表已导入并重新校验许可证。');
        return;
      }
      throw const FormatException('offline_license_invalid_format');
    });
  }

  Future<void> _clearLicense() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('清除离线许可证？'),
        content: const Text('清除后，本机批量处理和正式报告将重新按云端权益判断。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('清除'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    await _run(() async {
      await widget.appState.clearOfflineLicense();
      _showMessage('离线许可证已清除。');
    });
  }

  Future<void> _run(Future<void> Function() action) async {
    setState(() => _busy = true);
    try {
      await action();
    } catch (error) {
      _showMessage(_licenseErrorMessage(error));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  void _showMessage(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }
}

class _LicenseStatus extends StatelessWidget {
  const _LicenseStatus({required this.snapshot});

  final OfflineLicenseSnapshot snapshot;

  @override
  Widget build(BuildContext context) {
    final installationId = snapshot.installationId;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            HsStatusChip(label: _licenseStatusLabel(snapshot.status)),
            const Spacer(),
            if (snapshot.expiresAt != null)
              Text(
                '到期 ${_formatDate(snapshot.expiresAt!)}',
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: HsColors.textMuted),
              ),
          ],
        ),
        const SizedBox(height: 10),
        if (installationId.isNotEmpty)
          ListTile(
            contentPadding: EdgeInsets.zero,
            dense: true,
            title: const Text('安装实例 ID'),
            subtitle: SelectableText(installationId),
            trailing: IconButton(
              tooltip: '复制',
              onPressed: () =>
                  Clipboard.setData(ClipboardData(text: installationId)),
              icon: const Icon(Icons.copy_outlined),
            ),
          ),
        if (snapshot.licenseId != null)
          _StatusLine(label: '许可证', value: snapshot.licenseId!),
        if (snapshot.productCode != null)
          _StatusLine(label: '产品', value: snapshot.productCode!),
        if (snapshot.revocationListId != null)
          _StatusLine(
            label: '撤销列表',
            value:
                '${snapshot.revocationListId} / #${snapshot.revocationSequence}',
          ),
        if (snapshot.lastError != null)
          Text(
            _licenseCodeMessage(snapshot.lastError!),
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: HsColors.warning),
          ),
      ],
    );
  }
}

class _StatusLine extends StatelessWidget {
  const _StatusLine({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 72,
            child: Text(
              label,
              style: Theme.of(
                context,
              ).textTheme.bodySmall?.copyWith(color: HsColors.textMuted),
            ),
          ),
          Expanded(child: SelectableText(value)),
        ],
      ),
    );
  }
}

class _ActivationRequestDialog extends StatelessWidget {
  const _ActivationRequestDialog({required this.token});

  final String token;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('HSREQ1 激活请求'),
      content: SizedBox(
        width: 420,
        child: SingleChildScrollView(
          child: Column(
            children: [
              Container(
                color: Colors.white,
                padding: const EdgeInsets.all(12),
                child: QrImageView(data: token, size: 260),
              ),
              const SizedBox(height: 12),
              SelectableText(
                token,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton.icon(
          onPressed: () => Clipboard.setData(ClipboardData(text: token)),
          icon: const Icon(Icons.copy_outlined),
          label: const Text('复制'),
        ),
        TextButton.icon(
          onPressed: () => SharePlus.instance.share(
            ShareParams(
              text: token,
              subject: 'HiddenShield 离线激活请求',
              files: [
                XFile.fromData(
                  utf8.encode(token),
                  mimeType: 'text/plain',
                  name: 'hidden-shield-activation.hsreq',
                ),
              ],
              fileNameOverrides: const ['hidden-shield-activation.hsreq'],
            ),
          ),
          icon: const Icon(Icons.ios_share_outlined),
          label: const Text('分享'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('完成'),
        ),
      ],
    );
  }
}

class _OfflineLicenseScannerPage extends StatefulWidget {
  const _OfflineLicenseScannerPage();

  @override
  State<_OfflineLicenseScannerPage> createState() =>
      _OfflineLicenseScannerPageState();
}

class _OfflineLicenseScannerPageState
    extends State<_OfflineLicenseScannerPage> {
  bool _handled = false;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('扫描离线载荷')),
      body: MobileScanner(
        onDetect: (capture) {
          if (_handled) return;
          for (final barcode in capture.barcodes) {
            final raw = barcode.rawValue?.trim();
            if (raw == null ||
                (!raw.startsWith('HSLIC1.') && !raw.startsWith('HSRVL1.'))) {
              continue;
            }
            _handled = true;
            Navigator.pop(context, raw);
            return;
          }
        },
      ),
    );
  }
}

String _licenseStatusLabel(OfflineLicenseStatus status) => switch (status) {
  OfflineLicenseStatus.active => '离线授权有效',
  OfflineLicenseStatus.inactive => '未激活',
  OfflineLicenseStatus.notYetValid => '尚未生效',
  OfflineLicenseStatus.expired => '已过期',
  OfflineLicenseStatus.revoked => '已撤销',
  OfflineLicenseStatus.deviceMismatch => '设备不匹配',
  OfflineLicenseStatus.invalid => '许可证无效',
  OfflineLicenseStatus.secureStoreFailure => '安全存储不可用',
  OfflineLicenseStatus.unsupported => '当前平台不支持',
};

String _licenseErrorMessage(Object error) {
  if (error is FormatException) {
    return _licenseCodeMessage(error.message.toString());
  }
  return _licenseCodeMessage(error.toString());
}

String _licenseCodeMessage(String code) => switch (code) {
  'offline_license_unknown_key' => '当前构建未配置该许可证公钥，已拒绝激活。',
  'offline_license_signature_invalid' => '许可证签名无效。',
  'offline_license_device_mismatch' => '许可证绑定到另一安装实例，不能在本机使用。',
  'offline_license_not_yet_valid' => '许可证尚未生效。',
  'offline_license_expired' => '许可证已过期。',
  'offline_license_revoked' => '许可证已被导入的撤销列表撤销。',
  'offline_license_secure_storage_unavailable' =>
    '平台安全存储不可用；Web 和非 Android/iOS 平台会关闭离线授权。',
  'offline_license_revocation_signature_invalid' => '撤销列表签名无效。',
  _ => '离线载荷格式无效或无法验证。',
};

String _formatDate(DateTime value) {
  final local = value.toLocal();
  String two(int number) => number.toString().padLeft(2, '0');
  return '${local.year}-${two(local.month)}-${two(local.day)}';
}
