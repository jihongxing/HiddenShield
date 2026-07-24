import 'package:flutter/material.dart';
import 'package:hidden_shield_mobile/app/theme.dart';
import 'package:hidden_shield_mobile/shared/theme/design_tokens.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final scenario = _VisualScenario.fromName(
    Uri.base.queryParameters['scenario'] ?? 'free-unpaid',
  );
  runApp(_MobileVisualQaApp(scenario: scenario));
}

class _MobileVisualQaApp extends StatelessWidget {
  const _MobileVisualQaApp({required this.scenario});

  final _VisualScenario scenario;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield Mobile Visual QA',
      theme: buildHiddenShieldTheme(),
      home: _ScenarioScreen(scenario: scenario),
    );
  }
}

class _ScenarioScreen extends StatelessWidget {
  const _ScenarioScreen({required this.scenario});

  final _VisualScenario scenario;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Row(
          children: [
            Container(
              width: 30,
              height: 30,
              decoration: BoxDecoration(
                color: HsColors.accent,
                borderRadius: BorderRadius.circular(HsRadii.card),
                border: Border.all(color: HsColors.border),
              ),
              child: const Icon(
                Icons.shield_outlined,
                size: 18,
                color: HsColors.background,
              ),
            ),
            const SizedBox(width: HsSpacing.sm),
            const Text('HiddenShield'),
          ],
        ),
        centerTitle: false,
        actions: const [
          IconButton(
            tooltip: '订阅与权益',
            onPressed: null,
            icon: Icon(Icons.workspace_premium_outlined),
          ),
        ],
      ),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.fromLTRB(
            HsSpacing.lg,
            HsSpacing.lg,
            HsSpacing.lg,
            HsSpacing.xxl,
          ),
          children: [
            _HeaderBlock(scenario: scenario),
            const SizedBox(height: HsSpacing.lg),
            _EntitlementCard(scenario: scenario),
            const SizedBox(height: HsSpacing.md),
            _VaultRecordCard(scenario: scenario),
            const SizedBox(height: HsSpacing.md),
            _ReportStateCard(scenario: scenario),
            const SizedBox(height: HsSpacing.md),
            _PaymentStateCard(scenario: scenario),
            const SizedBox(height: HsSpacing.md),
            const _BoundaryCard(),
          ],
        ),
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: 2,
        backgroundColor: HsColors.navigation,
        indicatorColor: HsColors.chip,
        destinations: const [
          NavigationDestination(
            icon: Icon(Icons.dashboard_outlined),
            selectedIcon: Icon(Icons.dashboard),
            label: '工作台',
          ),
          NavigationDestination(
            icon: Icon(Icons.search_outlined),
            selectedIcon: Icon(Icons.search),
            label: '验证',
          ),
          NavigationDestination(
            icon: Icon(Icons.folder_outlined),
            selectedIcon: Icon(Icons.folder),
            label: '版权库',
          ),
          NavigationDestination(
            icon: Icon(Icons.view_list_outlined),
            selectedIcon: Icon(Icons.view_list),
            label: '批量',
          ),
          NavigationDestination(
            icon: Icon(Icons.settings_outlined),
            selectedIcon: Icon(Icons.settings),
            label: '设置',
          ),
        ],
      ),
    );
  }
}

class _HeaderBlock extends StatelessWidget {
  const _HeaderBlock({required this.scenario});

  final _VisualScenario scenario;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('版权库详情', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: HsSpacing.xs),
        Text(scenario.title, style: const TextStyle(color: HsColors.textMuted)),
      ],
    );
  }
}

class _EntitlementCard extends StatelessWidget {
  const _EntitlementCard({required this.scenario});

  final _VisualScenario scenario;

  @override
  Widget build(BuildContext context) {
    return _Panel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const _PanelTitle(
            icon: Icons.workspace_premium_outlined,
            title: '订阅与权益',
          ),
          const SizedBox(height: HsSpacing.md),
          Row(
            children: [
              Expanded(
                child: _Metric(label: '当前方案', value: scenario.planLabel),
              ),
              Expanded(
                child: _Metric(
                  label: '正式报告',
                  value: scenario.reportEntitlement,
                ),
              ),
            ],
          ),
          const SizedBox(height: HsSpacing.sm),
          _StatusPill(
            label: 'Free / Creator / Studio / Enterprise',
            tone: scenario.isCreator ? _Tone.ok : _Tone.neutral,
          ),
        ],
      ),
    );
  }
}

class _VaultRecordCard extends StatelessWidget {
  const _VaultRecordCard({required this.scenario});

  final _VisualScenario scenario;

  @override
  Widget build(BuildContext context) {
    return _Panel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const _PanelTitle(icon: Icons.image_outlined, title: '视觉迁移样张.png'),
          const SizedBox(height: HsSpacing.sm),
          const Text(
            'PREVIEW-QA-MOBILE-20260626',
            style: TextStyle(color: HsColors.text, fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: HsSpacing.md),
          const _DetailRow(label: '创作者身份', value: 'HiddenShield QA'),
          const _DetailRow(label: '完成后验证', value: '已通过'),
          const _DetailRow(label: '第三方验证', value: '未提交第三方验证'),
          const _DetailRow(label: '可信时间', value: '2026-06-26 09:20:00'),
          const SizedBox(height: HsSpacing.md),
          Row(
            children: [
              _StatusPill(label: '已同步', tone: _Tone.ok),
              const SizedBox(width: HsSpacing.sm),
              _StatusPill(label: '图片', tone: _Tone.neutral),
            ],
          ),
        ],
      ),
    );
  }
}

class _ReportStateCard extends StatelessWidget {
  const _ReportStateCard({required this.scenario});

  final _VisualScenario scenario;

  @override
  Widget build(BuildContext context) {
    final canExport = scenario.canExportReport;
    return _Panel(
      accent: canExport ? _Tone.ok : _Tone.warning,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _PanelTitle(
            icon: canExport
                ? Icons.picture_as_pdf_outlined
                : Icons.workspace_premium_outlined,
            title: canExport ? '导出正式报告' : 'Creator 正式报告',
          ),
          const SizedBox(height: HsSpacing.sm),
          Text(
            scenario.reportDetail,
            style: const TextStyle(color: HsColors.textMuted, height: 1.55),
          ),
          const SizedBox(height: HsSpacing.md),
          FilledButton.icon(
            onPressed: canExport ? () {} : null,
            icon: Icon(
              canExport ? Icons.picture_as_pdf_outlined : Icons.lock_outline,
            ),
            label: Text(canExport ? '导出正式报告' : '购买或开通后导出'),
          ),
          if (!scenario.isCreator && !scenario.canExportReport) ...[
            const SizedBox(height: HsSpacing.sm),
            OutlinedButton.icon(
              onPressed: scenario.canStartPurchase ? () {} : null,
              icon: const Icon(Icons.description_outlined),
              label: const Text('购买版权详细报告 · 19.9 元'),
            ),
            const SizedBox(height: HsSpacing.sm),
            OutlinedButton.icon(
              onPressed: scenario.canStartPurchase ? () {} : null,
              icon: const Icon(Icons.fact_check_outlined),
              label: const Text('购买维权证据包 · 49.9 元'),
            ),
          ],
        ],
      ),
    );
  }
}

class _PaymentStateCard extends StatelessWidget {
  const _PaymentStateCard({required this.scenario});

  final _VisualScenario scenario;

  @override
  Widget build(BuildContext context) {
    return _Panel(
      accent: scenario.paymentTone,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _PanelTitle(icon: scenario.paymentIcon, title: scenario.paymentTitle),
          const SizedBox(height: HsSpacing.sm),
          Text(
            scenario.paymentDetail,
            style: const TextStyle(color: HsColors.textMuted, height: 1.55),
          ),
          const SizedBox(height: HsSpacing.md),
          _StatusPill(
            label: scenario.paymentStatus,
            tone: scenario.paymentTone,
          ),
        ],
      ),
    );
  }
}

class _BoundaryCard extends StatelessWidget {
  const _BoundaryCard();

  @override
  Widget build(BuildContext context) {
    return const _Panel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _PanelTitle(icon: Icons.rule_folder_outlined, title: '能力边界'),
          SizedBox(height: HsSpacing.sm),
          Text(
            '报告是技术辅助材料，不构成法律意见或司法鉴定。L2 视频指纹存证不是视频画面盲水印；L3 本地或云端视频画面盲水印不开放。',
            style: TextStyle(color: HsColors.textMuted, height: 1.55),
          ),
        ],
      ),
    );
  }
}

class _Panel extends StatelessWidget {
  const _Panel({required this.child, this.accent = _Tone.neutral});

  final Widget child;
  final _Tone accent;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(HsSpacing.lg),
      decoration: BoxDecoration(
        color: switch (accent) {
          _Tone.ok => HsColors.surfaceRaised,
          _Tone.warning => HsColors.warningSurface,
          _Tone.danger => HsColors.dangerSurface,
          _Tone.neutral => HsColors.surfaceRaised,
        },
        borderRadius: BorderRadius.circular(HsRadii.card),
        border: Border.all(
          color: switch (accent) {
            _Tone.ok => HsColors.accent.withAlpha(80),
            _Tone.warning => HsColors.warning.withAlpha(90),
            _Tone.danger => HsColors.danger.withAlpha(90),
            _Tone.neutral => HsColors.border,
          },
        ),
      ),
      child: child,
    );
  }
}

class _PanelTitle extends StatelessWidget {
  const _PanelTitle({required this.icon, required this.title});

  final IconData icon;
  final String title;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Icon(icon, color: HsColors.accent),
        const SizedBox(width: HsSpacing.sm),
        Expanded(
          child: Text(
            title,
            style: const TextStyle(fontWeight: FontWeight.w700),
          ),
        ),
      ],
    );
  }
}

class _Metric extends StatelessWidget {
  const _Metric({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: const TextStyle(color: HsColors.textSubtle)),
        const SizedBox(height: HsSpacing.xs),
        Text(value, style: const TextStyle(fontWeight: FontWeight.w700)),
      ],
    );
  }
}

class _DetailRow extends StatelessWidget {
  const _DetailRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: HsSpacing.sm),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 96,
            child: Text(
              label,
              style: const TextStyle(color: HsColors.textMuted),
            ),
          ),
          Expanded(child: Text(value)),
        ],
      ),
    );
  }
}

class _StatusPill extends StatelessWidget {
  const _StatusPill({required this.label, required this.tone});

  final String label;
  final _Tone tone;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: switch (tone) {
          _Tone.ok => HsColors.accent.withAlpha(24),
          _Tone.warning => HsColors.warning.withAlpha(24),
          _Tone.danger => HsColors.danger.withAlpha(24),
          _Tone.neutral => HsColors.chip,
        },
        borderRadius: BorderRadius.circular(HsRadii.pill),
        border: Border.all(
          color: switch (tone) {
            _Tone.ok => HsColors.accent.withAlpha(80),
            _Tone.warning => HsColors.warning.withAlpha(90),
            _Tone.danger => HsColors.danger.withAlpha(90),
            _Tone.neutral => HsColors.border,
          },
        ),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: switch (tone) {
            _Tone.ok => HsColors.accent,
            _Tone.warning => HsColors.warning,
            _Tone.danger => HsColors.danger,
            _Tone.neutral => HsColors.textMuted,
          },
          fontSize: 12,
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }
}

enum _Tone { neutral, ok, warning, danger }

class _VisualScenario {
  const _VisualScenario({
    required this.name,
    required this.title,
    required this.planLabel,
    required this.reportEntitlement,
    required this.reportDetail,
    required this.paymentTitle,
    required this.paymentDetail,
    required this.paymentStatus,
    required this.paymentIcon,
    required this.paymentTone,
    this.isCreator = false,
    this.canExportReport = false,
    this.canStartPurchase = true,
  });

  final String name;
  final String title;
  final String planLabel;
  final String reportEntitlement;
  final String reportDetail;
  final String paymentTitle;
  final String paymentDetail;
  final String paymentStatus;
  final IconData paymentIcon;
  final _Tone paymentTone;
  final bool isCreator;
  final bool canExportReport;
  final bool canStartPurchase;

  static _VisualScenario fromName(String name) {
    return switch (name) {
      'free-paid' => _freePaid,
      'creator' => _creator,
      'payment-unconfigured' => _paymentUnconfigured,
      'refund-revoked' => _refundRevoked,
      _ => _freeUnpaid,
    };
  }
}

const _freeUnpaid = _VisualScenario(
  name: 'free-unpaid',
  title: 'Free 未购买单份报告',
  planLabel: '免费版',
  reportEntitlement: '未解锁',
  reportDetail: '当前记录尚未购买单份版权详细报告，也未开通 Creator。可复制基础存证摘要，正式报告需要购买或升级。',
  paymentTitle: '未创建支付会话',
  paymentDetail: 'Free 用户仍可查看版权库和基础摘要。购买只解锁当前记录，不改变订阅等级。',
  paymentStatus: '待购买',
  paymentIcon: Icons.lock_outline,
  paymentTone: _Tone.warning,
);

const _freePaid = _VisualScenario(
  name: 'free-paid',
  title: 'Free 已购买当前记录授权',
  planLabel: '免费版',
  reportEntitlement: '单记录已解锁',
  reportDetail: '单份版权详细报告授权有效，可导出当前记录正式报告；这不会打开 Creator 的全局 report_export 权益。',
  paymentTitle: '单份报告授权有效',
  paymentDetail: '授权只绑定当前版权记录和商品，退款或撤销后会回到未解锁状态。',
  paymentStatus: '已解锁',
  paymentIcon: Icons.verified_outlined,
  paymentTone: _Tone.ok,
  canExportReport: true,
);

const _creator = _VisualScenario(
  name: 'creator',
  title: 'Creator 订阅生效',
  planLabel: 'Creator',
  reportEntitlement: 'Creator 权益内',
  reportDetail: 'Creator 已开放正式报告、批量队列和正式云同步。导出报告不需要逐条购买。',
  paymentTitle: '订阅权益已生效',
  paymentDetail: '当前账户可使用 Creator 权益；云端视频画面盲水印 L3 仍不开放。',
  paymentStatus: '订阅有效',
  paymentIcon: Icons.workspace_premium_outlined,
  paymentTone: _Tone.ok,
  isCreator: true,
  canExportReport: true,
);

const _paymentUnconfigured = _VisualScenario(
  name: 'payment-unconfigured',
  title: '支付通道未配置',
  planLabel: '免费版',
  reportEntitlement: '未解锁',
  reportDetail: '当前记录可购买单份报告，但真实支付依赖微信商户参数和公网 HTTPS 回调配置。',
  paymentTitle: '支付通道尚未完成配置',
  paymentDetail: '未配置时不能完成真实付款，应展示明确提示，而不是把能力降级为未来功能。',
  paymentStatus: '通道未配置',
  paymentIcon: Icons.payments_outlined,
  paymentTone: _Tone.warning,
  canStartPurchase: false,
);

const _refundRevoked = _VisualScenario(
  name: 'refund-revoked',
  title: '退款撤销后授权失效',
  planLabel: '免费版',
  reportEntitlement: '已撤销',
  reportDetail: '单份报告授权已被退款或撤销，不再允许导出正式报告。用户可重新购买或升级 Creator。',
  paymentTitle: '授权已撤销',
  paymentDetail: '退款撤销只影响对应记录的单份报告授权，不改变 Free 订阅状态。',
  paymentStatus: '不可导出',
  paymentIcon: Icons.report_gmailerrorred_outlined,
  paymentTone: _Tone.danger,
);
