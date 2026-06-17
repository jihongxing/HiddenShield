import 'package:flutter/material.dart';

import '../theme/design_tokens.dart';

class ActionCard extends StatelessWidget {
  const ActionCard({
    super.key,
    required this.title,
    required this.icon,
    required this.description,
    this.onTap,
  });

  final String title;
  final IconData icon;
  final String description;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Card(
        elevation: 0,
        color: HsColors.surface,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(HsRadii.panel),
        ),
        child: InkWell(
          borderRadius: BorderRadius.circular(HsRadii.panel),
          onTap: onTap,
          child: ListTile(
            leading: Icon(icon, color: HsColors.accent),
            title: Text(title),
            subtitle: Text(description),
            trailing: const Icon(Icons.chevron_right),
          ),
        ),
      ),
    );
  }
}
