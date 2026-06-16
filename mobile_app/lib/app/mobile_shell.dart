import 'package:flutter/material.dart';

import '../bridge/watermark_bridge.dart';
import '../features/settings/settings_page.dart';
import '../features/verify/verify_page.dart';
import '../features/vault/vault_page.dart';
import '../features/workspace/workspace_page.dart';

class MobileShell extends StatefulWidget {
  const MobileShell({super.key, required this.bridge});

  final WatermarkBridge bridge;

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
    _NavTab(label: '取证', icon: Icons.search_outlined, activeIcon: Icons.search),
    _NavTab(
      label: '版权库',
      icon: Icons.folder_outlined,
      activeIcon: Icons.folder,
    ),
    _NavTab(
      label: '设置',
      icon: Icons.settings_outlined,
      activeIcon: Icons.settings,
    ),
  ];

  @override
  Widget build(BuildContext context) {
    final bridge = widget.bridge;
    return Scaffold(
      appBar: AppBar(
        title: const Text('HiddenShield'),
        centerTitle: false,
        backgroundColor: const Color(0xFF0F151B),
      ),
      body: SafeArea(
        child: IndexedStack(
          index: _currentIndex,
          children: [
            WorkspacePage(bridge: bridge),
            VerifyPage(bridge: bridge),
            VaultPage(bridge: bridge),
            SettingsPage(bridge: bridge),
          ],
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
  });

  final String label;
  final IconData icon;
  final IconData activeIcon;
}
