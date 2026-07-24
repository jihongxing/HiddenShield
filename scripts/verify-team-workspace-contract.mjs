import { readFileSync } from 'node:fs';

const sources = {
  desktopVault: readFileSync('src/views/VaultView.vue', 'utf8'),
  desktopSettings: readFileSync('src/components/SettingsPanel.vue', 'utf8'),
  mobileState: readFileSync('mobile_app/lib/app/mobile_app_state.dart', 'utf8'),
  mobileVault: readFileSync('mobile_app/lib/features/vault/vault_page.dart', 'utf8'),
  mobileSettings: readFileSync('mobile_app/lib/features/settings/settings_page.dart', 'utf8'),
  roadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  design: readFileSync('docs/Studio团队版权库模型设计.md', 'utf8'),
};

assert(
  sources.desktopVault.includes('team_workspace') &&
    sources.desktopVault.includes('团队空间') &&
    sources.desktopVault.includes('共享版权库'),
  'desktop vault must expose Studio team workspace entry gated by team_workspace',
);
assert(
  sources.desktopSettings.includes('canUseTeamWorkspace') &&
    sources.desktopSettings.includes('Studio 团队空间') &&
    sources.desktopSettings.includes('不共享媒体文件和本地路径'),
  'desktop settings must show Studio team workspace status and privacy boundary',
);
assert(
  sources.mobileState.includes('canUseTeamWorkspace') &&
    sources.mobileState.includes("entitlementFeatures['team_workspace'] == true"),
  'mobile app state must expose team_workspace entitlement gate',
);
assert(
  sources.mobileVault.includes('_TeamWorkspaceCard') &&
    sources.mobileVault.includes('团队空间') &&
    sources.mobileVault.includes('个人版权库'),
  'mobile vault must expose Studio team workspace entry without enabling management actions',
);
assert(
  sources.mobileSettings.includes('_TeamWorkspacePanel') &&
    sources.mobileSettings.includes('Studio 团队空间') &&
    sources.mobileSettings.includes('不共享媒体文件和本地路径'),
  'mobile settings must show Studio team workspace status and privacy boundary',
);
assert(
  sources.design.includes('team_workspace') &&
    sources.design.includes('team audit log') &&
    sources.design.includes('不共享'),
  'Studio team workspace design doc must define entitlement, audit log, and sharing boundary',
);
assert(
  sources.roadmap.includes('Studio 页面入口预留') &&
    sources.roadmap.includes('团队空间入口'),
  'roadmap must record Studio team workspace entry progress',
);

console.log('Team workspace contract OK');

function assert(condition, message) {
  if (!condition) {
    console.error(`Team workspace contract failed: ${message}`);
    process.exit(1);
  }
}
