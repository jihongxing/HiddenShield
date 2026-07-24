import 'package:flutter/material.dart';

import '../bridge/watermark_bridge.dart';
import '../features/settings/settings_page.dart';
import '../features/verify/verify_page.dart';
import '../features/vault/vault_page.dart';
import '../features/workspace/local_batch_page.dart';
import '../features/workspace/workspace_page.dart';
import '../shared/theme/design_tokens.dart';
import 'mobile_app_state.dart';

class MobileShell extends StatefulWidget {
  const MobileShell({super.key, required this.bridge, required this.appState});

  final WatermarkBridge bridge;
  final MobileAppState appState;

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
    ),
    _NavTab(label: '验证', icon: Icons.search_outlined, activeIcon: Icons.search),
    _NavTab(
      label: '版权库',
      icon: Icons.folder_outlined,
      activeIcon: Icons.folder,
    ),
    _NavTab(
      label: '批量',
      icon: Icons.view_list_outlined,
      activeIcon: Icons.view_list,
    ),
    _NavTab(
      label: '设置',
      icon: Icons.settings_outlined,
      activeIcon: Icons.settings,
    ),
  ];

  Future<void> _openAccountRecoverySettings() async {
    await widget.appState.prepareCloudRelogin();
    if (!mounted) return;
    setState(() => _currentIndex = 4);
  }

  @override
  Widget build(BuildContext context) {
    final bridge = widget.bridge;
    final appState = widget.appState;
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
        actions: [
          IconButton(
            tooltip: '订阅与权益',
            onPressed: () => setState(() => _currentIndex = 4),
            icon: const Icon(Icons.workspace_premium_outlined),
          ),
        ],
      ),
      body: SafeArea(
        child: IndexedStack(
          index: _currentIndex,
          children: [
            WorkspacePage(
              bridge: bridge,
              appState: appState,
              onOpenVault: () => setState(() => _currentIndex = 2),
              onOpenSettings: _openAccountRecoverySettings,
            ),
            VerifyPage(bridge: bridge, appState: appState),
            VaultPage(bridge: bridge, appState: appState),
            LocalBatchPage(
              bridge: bridge,
              appState: appState,
              showAppBar: false,
            ),
            SettingsPage(bridge: bridge, appState: appState),
          ],
        ),
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _currentIndex,
        onDestinationSelected: (index) => setState(() => _currentIndex = index),
        backgroundColor: HsColors.navigation,
        indicatorColor: HsColors.chip,
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
  });

  final String label;
  final IconData icon;
  final IconData activeIcon;
}
