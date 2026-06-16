import 'package:flutter/material.dart';

void main() {
  runApp(const HiddenShieldApp());
}

class HiddenShieldApp extends StatelessWidget {
  const HiddenShieldApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'HiddenShield',
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF1E6F66),
          brightness: Brightness.dark,
        ),
        scaffoldBackgroundColor: const Color(0xFF0C1116),
      ),
      home: const MobileShell(),
    );
  }
}

class MobileShell extends StatefulWidget {
  const MobileShell({super.key});

  @override
  State<MobileShell> createState() => _MobileShellState();
}

class _MobileShellState extends State<MobileShell> {
  int _currentIndex = 0;

  final List<_NavTab> _tabs = const [
    _NavTab(
      label: '工作台',
      icon: Icons.dashboard_outlined,
      activeIcon: Icons.dashboard,
      page: _WorkspacePage(),
    ),
    _NavTab(
      label: '取证',
      icon: Icons.search_outlined,
      activeIcon: Icons.search,
      page: _VerifyPage(),
    ),
    _NavTab(
      label: '版权库',
      icon: Icons.folder_outlined,
      activeIcon: Icons.folder,
      page: _VaultPage(),
    ),
    _NavTab(
      label: '设置',
      icon: Icons.settings_outlined,
      activeIcon: Icons.settings,
      page: _SettingsPage(),
    ),
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('HiddenShield'),
        centerTitle: false,
        backgroundColor: const Color(0xFF0F151B),
      ),
      body: SafeArea(
        child: IndexedStack(
          index: _currentIndex,
          children: _tabs.map((tab) => tab.page).toList(),
        ),
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _currentIndex,
        onDestinationSelected: (index) => setState(() => _currentIndex = index),
        backgroundColor: const Color(0xFF121920),
        destinations: _tabs
            .map(
              (tab) => NavigationDestination(
                icon: Icon(tab.icon),
                selectedIcon: Icon(tab.activeIcon),
                label: tab.label,
              ),
            )
            .toList(),
      ),
    );
  }
}

class _NavTab {
  const _NavTab({
    required this.label,
    required this.icon,
    required this.activeIcon,
    required this.page,
  });

  final String label;
  final IconData icon;
  final IconData activeIcon;
  final Widget page;
}

class _WorkspacePage extends StatelessWidget {
  const _WorkspacePage();

  @override
  Widget build(BuildContext context) {
    return _PageScaffold(
      title: '工作台',
      subtitle: '图片和音频的本地确权入口',
      children: const [
        _ActionCard(
          title: '图片嵌入',
          icon: Icons.image_outlined,
          description: '导入图片，生成带水印副本并写入版权库。',
        ),
        _ActionCard(
          title: '音频嵌入',
          icon: Icons.graphic_eq_outlined,
          description: '导入 WAV 音频，完成本地盲水印写入。',
        ),
        _ActionCard(
          title: '最近任务',
          icon: Icons.history_outlined,
          description: '这里会显示最近的处理结果和重写链路。',
        ),
      ],
    );
  }
}

class _VerifyPage extends StatelessWidget {
  const _VerifyPage();

  @override
  Widget build(BuildContext context) {
    return _PageScaffold(
      title: '取证',
      subtitle: '检测疑似侵权图片或音频，展示命中和链路',
      children: const [
        _ActionCard(
          title: '文件提取',
          icon: Icons.document_scanner_outlined,
          description: '选择文件后自动提取水印并匹配版权库。',
        ),
        _ActionCard(
          title: '结果摘要',
          icon: Icons.fact_check_outlined,
          description: '展示 UID、revision、父级 UID 和重写原因。',
        ),
      ],
    );
  }
}

class _VaultPage extends StatelessWidget {
  const _VaultPage();

  @override
  Widget build(BuildContext context) {
    return _PageScaffold(
      title: '版权库',
      subtitle: '时间线、详情和派生链',
      children: const [
        _ActionCard(
          title: '时间线',
          icon: Icons.timeline_outlined,
          description: '浏览记录，按时间查看每次写入。',
        ),
        _ActionCard(
          title: '链路详情',
          icon: Icons.device_hub_outlined,
          description: '查看 parent UID、revision 和 rewrite_reason。',
        ),
      ],
    );
  }
}

class _SettingsPage extends StatelessWidget {
  const _SettingsPage();

  @override
  Widget build(BuildContext context) {
    return _PageScaffold(
      title: '设置',
      subtitle: '身份、同步、隐私与帮助',
      children: const [
        _ActionCard(
          title: '创作者身份',
          icon: Icons.badge_outlined,
          description: '后续接入创作者身份包和桌面配对。',
        ),
        _ActionCard(
          title: '同步与备份',
          icon: Icons.sync_outlined,
          description: '后续接入桌面端同步状态和冲突处理。',
        ),
        _ActionCard(
          title: '隐私与权限',
          icon: Icons.lock_outline,
          description: '管理相册、文件、相机和通知权限。',
        ),
      ],
    );
  }
}

class _PageScaffold extends StatelessWidget {
  const _PageScaffold({
    required this.title,
    required this.subtitle,
    required this.children,
  });

  final String title;
  final String subtitle;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(
          title,
          style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                fontWeight: FontWeight.w700,
              ),
        ),
        const SizedBox(height: 8),
        Text(
          subtitle,
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                color: Colors.white70,
              ),
        ),
        const SizedBox(height: 16),
        const _StatusCard(),
        const SizedBox(height: 16),
        ...children,
      ],
    );
  }
}

class _StatusCard extends StatelessWidget {
  const _StatusCard();

  @override
  Widget build(BuildContext context) {
    return Card(
      elevation: 0,
      color: const Color(0xFF162028),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '本地优先 · 未配对桌面',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            const Text(
              '先完成图片和音频的本地确权，再接入桌面同步。',
              style: TextStyle(color: Colors.white70),
            ),
          ],
        ),
      ),
    );
  }
}

class _ActionCard extends StatelessWidget {
  const _ActionCard({
    required this.title,
    required this.icon,
    required this.description,
  });

  final String title;
  final IconData icon;
  final String description;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Card(
        elevation: 0,
        color: const Color(0xFF141B22),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
        child: ListTile(
          leading: Icon(icon, color: const Color(0xFF59D2C2)),
          title: Text(title),
          subtitle: Text(description),
          trailing: const Icon(Icons.chevron_right),
        ),
      ),
    );
  }
}
