import 'package:flutter/material.dart';
import 'package:hidden_shield_mobile/app/theme.dart';
import 'package:hidden_shield_mobile/shared/theme/design_tokens.dart';
import 'package:hidden_shield_mobile/shared/widgets/feature_page_scaffold.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final page = _FlowPage.fromName(Uri.base.queryParameters['page']);
  runApp(_MobileFlowVisualQaApp(page: page));
}

class _MobileFlowVisualQaApp extends StatelessWidget {
  const _MobileFlowVisualQaApp({required this.page});

  final _FlowPage page;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield Mobile Flow Visual QA',
      theme: buildHiddenShieldTheme(),
      home: _FlowShell(page: page),
    );
  }
}

class _FlowShell extends StatelessWidget {
  const _FlowShell({required this.page});

  final _FlowPage page;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: FeaturePageScaffold(
          title: page.title,
          subtitle: page.subtitle,
          icon: page.icon,
          trailing: _StatusPill(label: page.stateLabel, tone: page.tone),
          children: [
            _PrimaryPanel(page: page),
            const SizedBox(height: HsSpacing.md),
            ...page.sections.map(
              (section) => Padding(
                padding: const EdgeInsets.only(bottom: HsSpacing.md),
                child: _SectionPanel(section: section),
              ),
            ),
            const _CapabilityBoundaryPanel(),
          ],
        ),
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: page.navIndex,
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

class _PrimaryPanel extends StatelessWidget {
  const _PrimaryPanel({required this.page});

  final _FlowPage page;

  @override
  Widget build(BuildContext context) {
    return _Panel(
      accent: page.tone,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _PanelTitle(icon: page.primaryIcon, title: page.primaryTitle),
          const SizedBox(height: HsSpacing.sm),
          Text(
            page.primaryDetail,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: HsColors.textMuted,
              height: 1.5,
            ),
          ),
          const SizedBox(height: HsSpacing.lg),
          Wrap(
            spacing: HsSpacing.sm,
            runSpacing: HsSpacing.sm,
            children: page.metrics
                .map((metric) => _MetricChip(metric: metric))
                .toList(),
          ),
          const SizedBox(height: HsSpacing.lg),
          Row(
            children: [
              Expanded(
                child: FilledButton.icon(
                  onPressed: () {},
                  icon: Icon(page.primaryActionIcon),
                  label: Text(page.primaryAction),
                ),
              ),
              const SizedBox(width: HsSpacing.sm),
              IconButton.outlined(
                tooltip: page.secondaryAction,
                onPressed: () {},
                icon: Icon(page.secondaryActionIcon),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _SectionPanel extends StatelessWidget {
  const _SectionPanel({required this.section});

  final _FlowSection section;

  @override
  Widget build(BuildContext context) {
    return _Panel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _PanelTitle(icon: section.icon, title: section.title),
          const SizedBox(height: HsSpacing.md),
          ...section.rows.map(
            (row) => _DetailRow(label: row.label, value: row.value),
          ),
          if (section.actions.isNotEmpty) ...[
            const SizedBox(height: HsSpacing.md),
            Wrap(
              spacing: HsSpacing.sm,
              runSpacing: HsSpacing.sm,
              children: section.actions
                  .map(
                    (action) => OutlinedButton.icon(
                      onPressed: action.enabled ? () {} : null,
                      icon: Icon(action.icon),
                      label: Text(action.label),
                    ),
                  )
                  .toList(),
            ),
          ],
        ],
      ),
    );
  }
}

class _CapabilityBoundaryPanel extends StatelessWidget {
  const _CapabilityBoundaryPanel();

  @override
  Widget build(BuildContext context) {
    return const _Panel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          _PanelTitle(icon: Icons.rule_folder_outlined, title: '能力边界'),
          SizedBox(height: HsSpacing.sm),
          Text(
            'Web QA 入口只验证移动端视觉语言和状态排布，不产生正式版权记录。L1 是视频音轨水印，L2 是视频指纹存证，L3 视频画面盲水印不开放。',
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
            style: const TextStyle(
              color: HsColors.text,
              fontWeight: FontWeight.w800,
            ),
          ),
        ),
      ],
    );
  }
}

class _MetricChip extends StatelessWidget {
  const _MetricChip({required this.metric});

  final _Metric metric;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 148,
      padding: const EdgeInsets.all(HsSpacing.md),
      decoration: BoxDecoration(
        color: HsColors.surfaceMuted,
        borderRadius: BorderRadius.circular(HsRadii.card),
        border: Border.all(color: HsColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            metric.label,
            style: const TextStyle(color: HsColors.textSubtle, fontSize: 12),
          ),
          const SizedBox(height: HsSpacing.xs),
          Text(
            metric.value,
            style: const TextStyle(
              color: HsColors.text,
              fontWeight: FontWeight.w800,
            ),
          ),
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
          fontWeight: FontWeight.w800,
        ),
      ),
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
            width: 92,
            child: Text(
              label,
              style: const TextStyle(color: HsColors.textMuted),
            ),
          ),
          Expanded(
            child: Text(value, style: const TextStyle(color: HsColors.text)),
          ),
        ],
      ),
    );
  }
}

enum _Tone { neutral, ok, warning, danger }

class _FlowPage {
  const _FlowPage({
    required this.name,
    required this.navIndex,
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.stateLabel,
    required this.tone,
    required this.primaryIcon,
    required this.primaryTitle,
    required this.primaryDetail,
    required this.primaryAction,
    required this.primaryActionIcon,
    required this.secondaryAction,
    required this.secondaryActionIcon,
    required this.metrics,
    required this.sections,
  });

  final String name;
  final int navIndex;
  final String title;
  final String subtitle;
  final IconData icon;
  final String stateLabel;
  final _Tone tone;
  final IconData primaryIcon;
  final String primaryTitle;
  final String primaryDetail;
  final String primaryAction;
  final IconData primaryActionIcon;
  final String secondaryAction;
  final IconData secondaryActionIcon;
  final List<_Metric> metrics;
  final List<_FlowSection> sections;

  static _FlowPage fromName(String? name) {
    return switch (name) {
      'adaptive' => _adaptivePage,
      'image' => _imagePage,
      'audio' => _audioPage,
      'batch' => _batchPage,
      'verify' => _verifyPage,
      _ => _workspacePage,
    };
  }
}

class _Metric {
  const _Metric(this.label, this.value);

  final String label;
  final String value;
}

class _FlowSection {
  const _FlowSection({
    required this.icon,
    required this.title,
    required this.rows,
    this.actions = const [],
  });

  final IconData icon;
  final String title;
  final List<_Detail> rows;
  final List<_ActionSpec> actions;
}

class _Detail {
  const _Detail(this.label, this.value);

  final String label;
  final String value;
}

class _ActionSpec {
  const _ActionSpec(this.label, this.icon, {this.enabled = true});

  final String label;
  final IconData icon;
  final bool enabled;
}

const _workspacePage = _FlowPage(
  name: 'workspace',
  navIndex: 0,
  title: '工作台',
  subtitle: '从一个主动作进入图片、音频、视频音轨和视频指纹存证流程。',
  icon: Icons.dashboard_outlined,
  stateLabel: 'Creator 可用',
  tone: _Tone.ok,
  primaryIcon: Icons.auto_awesome_motion_outlined,
  primaryTitle: '今日处理概览',
  primaryDetail: '工作台聚合最近记录、批量队列、当前权益和创作者身份。移动端保持单列扫描，桌面端保持并行信息密度。',
  primaryAction: '开始处理',
  primaryActionIcon: Icons.add_circle_outline,
  secondaryAction: '打开版权库',
  secondaryActionIcon: Icons.folder_outlined,
  metrics: [
    _Metric('今日完成', '6 条'),
    _Metric('待验证', '2 条'),
    _Metric('批量队列', '1 个'),
    _Metric('正式报告', 'Creator'),
  ],
  sections: [
    _FlowSection(
      icon: Icons.workspace_premium_outlined,
      title: '当前权益',
      rows: [
        _Detail('方案', 'Creator'),
        _Detail('批量队列', '已开放'),
        _Detail('云同步', '已开放，不同步原始媒体'),
        _Detail('报告', '正式报告可导出'),
      ],
    ),
    _FlowSection(
      icon: Icons.person_outline,
      title: '创作者身份',
      rows: [
        _Detail('显示名', 'HiddenShield QA'),
        _Detail('输出目录', '已设置'),
        _Detail('可信时间', '第三方验证未提交时显示未记录'),
      ],
    ),
  ],
);

const _adaptivePage = _FlowPage(
  name: 'adaptive',
  navIndex: 0,
  title: '智能处理',
  subtitle: '按文件类型选择图片写入、音频写入、视频音轨水印或视频指纹存证。',
  icon: Icons.hub_outlined,
  stateLabel: '预检就绪',
  tone: _Tone.ok,
  primaryIcon: Icons.rule_outlined,
  primaryTitle: '文件预检',
  primaryDetail: '统一展示格式、时长、大小、重写风险和权益状态。短音频、未支持格式和 L2 权益不足都先给出可执行说明。',
  primaryAction: '选择文件',
  primaryActionIcon: Icons.upload_file_outlined,
  secondaryAction: '查看能力边界',
  secondaryActionIcon: Icons.info_outline,
  metrics: [
    _Metric('图片', 'PNG / JPEG / WebP'),
    _Metric('音频', '30 秒以上'),
    _Metric('视频 L1', '音轨水印'),
    _Metric('视频 L2', '指纹存证'),
  ],
  sections: [
    _FlowSection(
      icon: Icons.checklist_outlined,
      title: '预检结果',
      rows: [
        _Detail('格式', '可处理'),
        _Detail('重写风险', '未检测到已写入标记'),
        _Detail('权益', 'Creator 权益通过'),
        _Detail('下一步', '确认创作者身份后写入'),
      ],
    ),
    _FlowSection(
      icon: Icons.layers_outlined,
      title: '视频分层',
      rows: [
        _Detail('L1', '视频音轨水印可用'),
        _Detail('L2', '视频指纹存证按 Creator 权益开放'),
        _Detail('L3', '视频画面盲水印不开放'),
      ],
      actions: [_ActionSpec('L3 不开放', Icons.lock_outline, enabled: false)],
    ),
  ],
);

const _imagePage = _FlowPage(
  name: 'image',
  navIndex: 0,
  title: '图片写入',
  subtitle: '写入保护副本，完成后做回读验证并进入版权库。',
  icon: Icons.image_outlined,
  stateLabel: '可写入',
  tone: _Tone.ok,
  primaryIcon: Icons.image_search_outlined,
  primaryTitle: '图片写入主线',
  primaryDetail: '移动端展示文件选择、预检、写入、完成后验证和分享保护副本。无法取得真实路径时只展示保护副本名称。',
  primaryAction: '写入图片',
  primaryActionIcon: Icons.draw_outlined,
  secondaryAction: '分享保护副本',
  secondaryActionIcon: Icons.ios_share_outlined,
  metrics: [
    _Metric('格式', 'PNG'),
    _Metric('验证', '已通过'),
    _Metric('版权库', '已入库'),
    _Metric('同步', '元数据同步'),
  ],
  sections: [
    _FlowSection(
      icon: Icons.verified_outlined,
      title: '完成后验证',
      rows: [
        _Detail('版权编号', '预览样例'),
        _Detail('保护副本', 'visual-qa-output.png'),
        _Detail('验证状态', '已通过'),
        _Detail('本地路径', '移动端不伪造路径'),
      ],
      actions: [
        _ActionSpec('保存到文件', Icons.save_alt_outlined),
        _ActionSpec('进入版权库', Icons.folder_outlined),
      ],
    ),
    _FlowSection(
      icon: Icons.privacy_tip_outlined,
      title: '隐私边界',
      rows: [
        _Detail('原始图片', '不同步'),
        _Detail('保护副本', '不同步'),
        _Detail('云同步', '仅同步版权记录元数据'),
      ],
    ),
  ],
);

const _audioPage = _FlowPage(
  name: 'audio',
  navIndex: 0,
  title: '音频写入',
  subtitle: '30 秒以上音频可写入，保持声道和格式处理说明可解释。',
  icon: Icons.graphic_eq_outlined,
  stateLabel: '可写入',
  tone: _Tone.ok,
  primaryIcon: Icons.audiotrack_outlined,
  primaryTitle: '音频写入主线',
  primaryDetail: '预检先说明时长、格式归一化和声道处理。短片段不足时不承诺可验证，失败项不会进入版权库成功态。',
  primaryAction: '写入音频',
  primaryActionIcon: Icons.library_music_outlined,
  secondaryAction: '查看预检',
  secondaryActionIcon: Icons.fact_check_outlined,
  metrics: [
    _Metric('时长', '03:42'),
    _Metric('声道', '双声道'),
    _Metric('验证', '已通过'),
    _Metric('报告', '可导出'),
  ],
  sections: [
    _FlowSection(
      icon: Icons.tune_outlined,
      title: '音频预检',
      rows: [
        _Detail('格式', 'WAV / MP3 / FLAC / OGG / M4A'),
        _Detail('最短时长', '30 秒'),
        _Detail('短片段', '不作为产品承诺'),
        _Detail('声道', '不承诺静默改成单声道'),
      ],
    ),
    _FlowSection(
      icon: Icons.verified_user_outlined,
      title: '写入结果',
      rows: [
        _Detail('保护副本', 'visual-qa-output.wav'),
        _Detail('验证状态', '已通过'),
        _Detail('版权库', '已生成记录'),
      ],
    ),
  ],
);

const _batchPage = _FlowPage(
  name: 'batch',
  navIndex: 3,
  title: '批量队列',
  subtitle: '本地批量是 Creator 权益，队列状态、失败重试和门禁双端一致。',
  icon: Icons.view_list_outlined,
  stateLabel: 'Creator',
  tone: _Tone.ok,
  primaryIcon: Icons.playlist_add_check_outlined,
  primaryTitle: '队列运行中',
  primaryDetail: '批量页以队列呈现，不用杂乱表格。每个任务展示进度、验证结果、失败原因和恢复动作。',
  primaryAction: '继续队列',
  primaryActionIcon: Icons.play_arrow_outlined,
  secondaryAction: '暂停',
  secondaryActionIcon: Icons.pause_outlined,
  metrics: [
    _Metric('总数', '18'),
    _Metric('完成', '12'),
    _Metric('失败', '1'),
    _Metric('待处理', '5'),
  ],
  sections: [
    _FlowSection(
      icon: Icons.lock_open_outlined,
      title: '权益门禁',
      rows: [
        _Detail('Free', '不能创建正式批量任务'),
        _Detail('Creator', '可创建本地批量队列'),
        _Detail('云端批量', '与本地批量分开计费'),
      ],
    ),
    _FlowSection(
      icon: Icons.error_outline,
      title: '失败项处理',
      rows: [
        _Detail('失败原因', '格式不支持'),
        _Detail('恢复动作', '重试或移出队列'),
        _Detail('版权库', '失败项不进入成功态'),
      ],
      actions: [
        _ActionSpec('重试失败项', Icons.refresh_outlined),
        _ActionSpec('取消队列', Icons.cancel_outlined),
      ],
    ),
  ],
);

const _verifyPage = _FlowPage(
  name: 'verify',
  navIndex: 1,
  title: '验证',
  subtitle: '先给验证结果，再给证据摘要、匹配记录和后续动作。',
  icon: Icons.search_outlined,
  stateLabel: '可信命中',
  tone: _Tone.ok,
  primaryIcon: Icons.verified_outlined,
  primaryTitle: '验证结果',
  primaryDetail: '验证页对图片、音频、视频音轨和 L2 指纹存证保持同口径。疑似命中和可信命中都解释置信度来源。',
  primaryAction: '导出摘要',
  primaryActionIcon: Icons.summarize_outlined,
  secondaryAction: '复制结果',
  secondaryActionIcon: Icons.copy_outlined,
  metrics: [
    _Metric('置信度', '0.94'),
    _Metric('匹配', '1 条'),
    _Metric('时间', '已记录'),
    _Metric('报告', '可导出'),
  ],
  sections: [
    _FlowSection(
      icon: Icons.manage_search_outlined,
      title: '证据摘要',
      rows: [
        _Detail('匹配记录', '视觉 QA 样例'),
        _Detail('完成后验证', '已通过'),
        _Detail('第三方验证', '未提交第三方验证'),
        _Detail('可信时间', '2026-06-26 09:20:00'),
      ],
    ),
    _FlowSection(
      icon: Icons.description_outlined,
      title: '正式报告',
      rows: [
        _Detail('Creator', '可直接导出'),
        _Detail('Free', '可按当前记录购买单份报告'),
        _Detail('法律边界', '不构成法律意见或司法鉴定'),
      ],
      actions: [
        _ActionSpec('导出正式报告', Icons.picture_as_pdf_outlined),
        _ActionSpec('购买维权证据包', Icons.fact_check_outlined),
      ],
    ),
  ],
);
