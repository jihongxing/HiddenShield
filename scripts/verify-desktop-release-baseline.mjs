import { readFileSync } from "node:fs";

const sources = {
  commercialRoadmap: readFileSync("docs/商业化落地Roadmap.md", "utf8"),
  dualRoadmap: readFileSync("docs/双端能力一致性Roadmap.md", "utf8"),
  corePlan: readFileSync("docs/共享水印核心与跨端互验推进计划.md", "utf8"),
  capabilityBoundary: readFileSync("docs/当前真实能力边界说明.md", "utf8"),
  coreCapability: readFileSync("docs/watermark-core能力说明.md", "utf8"),
  app: readFileSync("src/App.vue", "utf8"),
  appShell: readFileSync("src/components/shell/AppShell.vue", "utf8"),
  styles: readFileSync("src/styles.css", "utf8"),
  dropZone: readFileSync("src/components/DropZone.vue", "utf8"),
  help: readFileSync("src/components/HelpCenter.vue", "utf8"),
  legal: readFileSync("src/content/legal.ts", "utf8"),
  proBadge: readFileSync("src/components/ProBadge.vue", "utf8"),
  workbench: readFileSync("src/views/WorkbenchView.vue", "utf8"),
  verify: readFileSync("src/views/VerifyView.vue", "utf8"),
  vault: readFileSync("src/views/VaultView.vue", "utf8"),
  overview: readFileSync("src/views/WorkbenchOverviewView.vue", "utf8"),
  settings: readFileSync("src/components/SettingsPanel.vue", "utf8"),
  subscription: readFileSync("src/components/SubscriptionPanel.vue", "utf8"),
  offlineLicensePanel: readFileSync("src/components/OfflineLicensePanel.vue", "utf8"),
  aiContentMarker: readFileSync("src/components/AIContentMarker.vue", "utf8"),
  progressPanel: readFileSync("src/components/ProgressPanel.vue", "utf8"),
  resultPage: readFileSync("src/components/ResultPage.vue", "utf8"),
  copyrightCard: readFileSync("src/components/CopyrightCard.vue", "utf8"),
  entitlements: readFileSync("src-tauri/src/entitlements.rs", "utf8"),
  commercialContract: readFileSync("docs/商业化契约与权益模型.md", "utf8"),
  offlineLicenseDesign: readFileSync("docs/CDKEY离线激活与本地许可证设计.md", "utf8"),
  workspaceContext: readFileSync("src/lib/workspace-context.ts", "utf8"),
  userFacingErrors: readFileSync("src/lib/user-facing-errors.ts", "utf8"),
  tauriApi: readFileSync("src/lib/tauri-api.ts", "utf8"),
  tauriConfig: readFileSync("src-tauri/tauri.conf.json", "utf8"),
  packageJson: readFileSync("package.json", "utf8"),
  installerSelfContainedGate: readFileSync(
    "scripts/run-desktop-installer-self-contained-gate.mjs",
    "utf8",
  ),
};

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

const copyrightSummarySource = sources.tauriApi.slice(
  sources.tauriApi.indexOf("export function buildCopyrightSummary"),
  sources.tauriApi.indexOf("export function formatOutputStrategy"),
);

for (const [name, source] of Object.entries({
  commercialRoadmap: sources.commercialRoadmap,
  dualRoadmap: sources.dualRoadmap,
  corePlan: sources.corePlan,
  capabilityBoundary: sources.capabilityBoundary,
  coreCapability: sources.coreCapability,
})) {
  assert(source.includes("2026-07-16"), `${name} must record the current release baseline date`);
}

assert(
  sources.commercialRoadmap.includes("冻结全部移动端") &&
    sources.commercialRoadmap.includes("桌面端与后端云服务") &&
    sources.commercialRoadmap.includes("桌面端发布必须完成离线验证") &&
    sources.commercialRoadmap.includes("服务方注册码生成") &&
    sources.commercialRoadmap.includes("屏蔽桌面端全部视频能力入口") &&
    sources.commercialRoadmap.includes("RC Gate") &&
    sources.commercialRoadmap.includes("GA Gate") &&
    sources.commercialRoadmap.includes("新商业权益基线已冻结") &&
    sources.commercialRoadmap.includes("RC 需按新映射重建复验") &&
    sources.commercialRoadmap.includes("HiddenShieldReleaseQA"),
  "commercial roadmap must define all five release baseline items",
);

for (const [name, source] of Object.entries({
  dualRoadmap: sources.dualRoadmap,
  corePlan: sources.corePlan,
  capabilityBoundary: sources.capabilityBoundary,
  coreCapability: sources.coreCapability,
})) {
  assert(
    source.includes("RC Gate") &&
      source.includes("GA Gate"),
    `${name} must define RC/GA gates`,
  );
}

assert(
  sources.dropZone.includes('extensions: [\n            "wav", "mp3", "aac", "flac", "ogg", "m4a",') &&
    sources.dropZone.includes('accept=".wav,.mp3,.aac,.flac,.ogg,.m4a,.jpg,.jpeg,.png,.bmp,.gif,.webp,.tiff"') &&
    sources.dropZone.includes("当前发布版本仅开放图片和音频，视频能力已暂停。"),
  "desktop file picker must hide video and drag-drop must reject it",
);

assert(
  sources.workbench.includes('sourceMeta.value.fileType === "video"') &&
    sources.workbench.includes('trackFeatureEvent("source_probe", "diagnostic"') &&
    sources.workbench.includes("(isImage.value || isAudio.value)"),
  "workbench must fail closed if a video path bypasses the picker",
);

assert(
  sources.workbench.includes("选择图片或音频") &&
    !sources.workbench.includes("选择图片、音频或视频") &&
    sources.workbench.includes('v-if="false && isVideo"') &&
    sources.verify.includes("当前版本仅支持图片和音频验证，暂不提供视频文件验证。"),
  "desktop process and verification pages must not expose video capability",
);

assert(
  !sources.workbench.includes("await refreshRewriteInspection(path, requestId)") &&
    sources.workbench.includes("这是已有作品的新版") &&
    !sources.workbench.includes("作为新版写入") &&
    !sources.userFacingErrors.includes("作为新版写入") &&
    sources.workbench.includes("普通作品无需提前检查") &&
    sources.workbench.includes("watch("),
  "rewrite preflight must run only when the user opts into a new revision",
);

assert(
  sources.aiContentMarker.includes('<details class="ai-toggle" open>') &&
    sources.progressPanel.includes('class="progress-steps"') &&
    sources.progressPanel.includes("读取作品") &&
    sources.progressPanel.includes("保存版权记录") &&
    !sources.progressPanel.includes('class="progress-panel__track"'),
  "work declaration must default open and processing progress must use product steps",
);

assert(
  !sources.verify.includes("Phase R4") &&
    sources.verify.includes("校验维权证据包") &&
    sources.verify.includes("校验过程不会修改文件"),
  "evidence pack verification must use product wording instead of roadmap phases",
);

assert(
  sources.overview.includes("离线能力") &&
    !sources.overview.includes("<strong>发布 Gate</strong>"),
  "release Gate terminology must stay out of product UI",
);

assert(
  !sources.app.includes("buildWorkspaceContext") &&
    !sources.app.includes(":context=") &&
    !sources.app.includes("workspaceContext") &&
    !sources.appShell.includes("ContextPanel") &&
    !sources.appShell.includes("context: WorkspaceContext") &&
    !sources.appShell.includes("<ContextPanel") &&
    sources.styles.includes("grid-template-columns: minmax(0, 1fr);") &&
    !sources.styles.includes(".hs-context-panel") &&
    !sources.workspaceContext.includes("WorkspaceContext") &&
    !sources.workspaceContext.includes("buildWorkspaceContext") &&
    sources.workspaceContext.includes("export function planLabel") &&
    sources.subscription.includes(".subscription-overlay--embedded .subscription-panel") &&
    sources.subscription.includes("width: 100% !important;"),
  "desktop shell must use a single main workspace without the removed context panel",
);

assert(
  sources.vault.includes("报告与维权服务") &&
    sources.vault.includes("¥19.9") &&
    sources.vault.includes("¥49.9") &&
    sources.vault.includes("补充可信时间") &&
    sources.vault.includes("本机创建时间（非第三方证明）") === false,
  "vault must emphasize per-record products and trusted-time remediation",
);

for (const [name, source] of Object.entries({
  overview: sources.overview,
  settings: sources.settings,
  subscription: sources.subscription,
  workspaceContext: sources.workspaceContext,
})) {
  assert(!source.includes("L2 视频存证"), `${name} must not expose L2 video notary`);
  assert(!source.includes("开始视频音轨写入"), `${name} must not expose L1 video processing`);
}

assert(
  sources.subscription.includes('["cloud_sync", "batch_processing"].includes(key)') &&
    sources.settings.includes('["cloud_sync", "batch_processing"].includes(key)'),
  "desktop entitlement summaries must expose only current baseline features",
);

assert(
    sources.subscription.includes('type PlanCode = "free" | "creator"') &&
    sources.subscription.includes('createBillingPaymentSession(plan, "yearly"') &&
    sources.subscription.includes("未付费 / 图片音频年费") &&
    sources.subscription.includes("未付费和年费已激活用户都必须按记录单独购买正式报告"),
  "subscription panel must expose only unpaid and yearly image/audio base entitlement",
);

assert(
  /features\s*\.insert\(LOCAL_BATCH_FEATURE\.to_string\(\), true\)/.test(sources.entitlements) &&
    sources.entitlements.includes('features.insert("report_export".to_string(), false)') &&
    !sources.entitlements.includes('features.insert(LOCAL_REPORT_FEATURE.to_string(), true)') &&
    sources.entitlements.includes('"图片/音频年费授权"'),
  "HSLIC1 must grant local batch only and must not grant report_export",
);

assert(
  sources.offlineLicensePanel.includes("一年期注册码只开放本地批量处理") &&
    sources.offlineLicensePanel.includes("正式报告始终单独购买"),
  "offline license panel must state the annual batch-only mapping",
);

for (const [name, source] of Object.entries({
  commercialContract: sources.commercialContract,
  offlineLicenseDesign: sources.offlineLicenseDesign,
})) {
  assert(
    source.includes("图片 / 音频年度基础权益") &&
      source.includes('"report_export": false') &&
      source.includes("记录级") &&
      source.includes("未来视频"),
    `${name} must freeze the new annual base, per-record report, and future-video model`,
  );
}

assert(
  !sources.app.includes("EnterpriseAuditView") &&
    !sources.app.includes("enterpriseAudit") &&
    !sources.workspaceContext.includes("Enterprise 内部管理") &&
    !sources.settings.includes("Studio 团队空间"),
  "desktop product must not expose Enterprise or legacy team-plan entries",
);

assert(
  sources.tauriConfig.includes('"frontendDist": "../dist"') &&
    sources.tauriConfig.includes('"type": "offlineInstaller"') &&
    sources.packageJson.includes(
      '"tauri:build": "tauri build -- --bin hidden_shield"',
    ) &&
    sources.packageJson.includes(
      '"tauri:build:no-bundle": "tauri build --no-bundle -- --bin hidden_shield"',
    ) &&
    sources.installerSelfContainedGate.includes("localhost:1420") &&
    sources.installerSelfContainedGate.includes("offlineInstaller") &&
    sources.installerSelfContainedGate.includes("_x64-setup") &&
    sources.installerSelfContainedGate.includes("uiAutomation"),
  "desktop installers must embed frontend assets and include the self-contained startup Gate",
);

assert(
  copyrightSummarySource.includes("【HiddenShield 本地版权记录摘要】") &&
    copyrightSummarySource.includes(
      "记录性质: HiddenShield 本地版权记录，非第三方公证、官方登记或法律权属结论",
    ) &&
    copyrightSummarySource.includes("创作者显示名称:") &&
    copyrightSummarySource.includes("身份信息来源: 用户本地声明") &&
    copyrightSummarySource.includes("身份核验状态: 未进行实名认证"),
  "basic copyright summary must use the frozen P0 title and identity boundary wording",
);

assert(
  copyrightSummarySource.includes("保护副本验证:") &&
    copyrightSummarySource.includes(
      "已从保护副本回读并验证版权编号，可由 HiddenShield 再次读取验证",
    ) &&
    copyrightSummarySource.includes("处理完成时间:") &&
    copyrightSummarySource.includes("第三方时间证明: 未获取") &&
    copyrightSummarySource.includes("网络授时时间:") &&
    copyrightSummarySource.includes("时间证明服务:"),
  "basic copyright summary must separate verification and time evidence semantics",
);

assert(
  copyrightSummarySource.includes("水印协议版本: V${record.payloadProtocolVersion}") &&
    copyrightSummarySource.includes("载荷完整性校验: ${formatPayloadAuthStatus(record.payloadAuthStatus)}") &&
    copyrightSummarySource.includes("版权编号生成方式:") &&
    copyrightSummarySource.includes("联网登记状态:") &&
    copyrightSummarySource.includes("登记收据编号:"),
  "basic copyright summary must use the frozen P0 protocol and registry labels",
);

assert(
  copyrightSummarySource.includes("原文件名:") &&
    copyrightSummarySource.includes("作品指纹（SHA-256）:") &&
    copyrightSummarySource.includes("保护副本摘要（SHA-256）:") &&
    copyrightSummarySource.includes("未生成保护副本") &&
    copyrightSummarySource.includes(
      "以下内容由创作者声明，HiddenShield 不进行 AI 生成检测、真实性鉴定或法律授权判断。",
    ),
  "basic copyright summary must use the frozen P0 file and declaration wording",
);

assert(
  copyrightSummarySource.includes(
    "本摘要在本地生成；原始媒体和保护副本未上传。版权元数据是否同步，以当前同步设置和联网登记状态为准。",
  ) &&
    copyrightSummarySource.includes('.filter((line) => line.trim().length > 0)') &&
    copyrightSummarySource.includes("formatCopyrightDateTime"),
  "basic copyright summary must filter blank lines and use its stable P0 footer and time formatter",
);

for (const forbiddenText of [
  "【隐盾版权存证】",
  "创作者身份:",
  "保护副本可取证",
  "Payload 协议:",
  "Payload 认证状态:",
  "登记收据: ${record.watermarkIdRegistryReceipt || \"未记录\"}",
  "可信时间: ${formatTrustedTime(record)}",
  "本存证由 HiddenShield 本地生成，数据未上传至任何服务器。",
]) {
  assert(
    !copyrightSummarySource.includes(forbiddenText),
    `basic copyright summary must remove legacy wording: ${forbiddenText}`,
  );
}

assert(
  sources.resultPage.includes('const isAudio = computed(() => props.sourceMeta.fileType === "audio")') &&
    sources.resultPage.includes("音频时长") &&
    sources.resultPage.includes("formatAudioDuration") &&
    sources.resultPage.includes("保护副本采用可稳定验证的音频格式，因此文件可能明显增大。") &&
    sources.resultPage.includes('v-if="!isAudio"') &&
    sources.resultPage.includes('v-if="sourceMeta.fileType === \'video\'"'),
  "audio result UI must hide image/video metrics and show duration plus file size",
);

assert(
  sources.resultPage.includes("水印协议版本") &&
    sources.resultPage.includes("版权编号生成方式") &&
    sources.resultPage.includes("联网登记状态") &&
    sources.resultPage.includes("载荷完整性校验") &&
    sources.resultPage.includes("保护副本验证") &&
    sources.resultPage.includes(
      "已从保护副本回读并验证版权编号，可由 HiddenShield 再次读取验证",
    ) &&
    !sources.resultPage.includes("<span>Payload 协议</span>") &&
    !sources.resultPage.includes("<span>Payload 认证</span>"),
  "processing result evidence must use the P0 product field projection",
);

assert(
  sources.copyrightCard.includes("<span>本地版权记录</span>") &&
    sources.copyrightCard.includes("<span>创作者显示名称</span>") &&
    sources.copyrightCard.includes("<span>身份信息来源</span>") &&
    sources.copyrightCard.includes("<span>身份核验状态</span>") &&
    sources.copyrightCard.includes("<span>保护副本验证</span>") &&
    sources.copyrightCard.includes("<span>联网登记状态</span>") &&
    sources.copyrightCard.includes("<span>第三方时间证明</span>") &&
    sources.copyrightCard.includes("<span>水印协议版本</span>") &&
    sources.copyrightCard.includes("<span>载荷完整性校验</span>") &&
    !sources.copyrightCard.includes("<span>创作者身份</span>") &&
    !sources.copyrightCard.includes("<span>可信时间来源</span>"),
  "copyright card must use the P0 identity, registry, verification, time, and technical wording",
);

assert(
  sources.vault.includes("<span>创作者显示名称</span>") &&
    sources.vault.includes("<span>版权编号生成方式</span>") &&
    sources.vault.includes("<span>联网登记状态</span>") &&
    sources.vault.includes("<span>第三方时间证明</span>") &&
    sources.vault.includes("<span>水印协议版本</span>") &&
    sources.vault.includes("<span>载荷完整性校验</span>") &&
    !sources.vault.includes("<span>创作者身份</span>") &&
    !sources.vault.includes("<span>Payload 协议</span>") &&
    !sources.vault.includes("<span>Payload 认证</span>") &&
    !sources.vault.includes("<span>可信时间来源</span>"),
  "vault record details must use the same P0 product field projection",
);

assert(
  sources.tauriApi.includes("export function formatCopyrightDateTime") &&
    sources.tauriApi.includes("export function formatTimeProofService") &&
    sources.tauriApi.includes('host === "freetsa.org"') &&
    sources.tauriApi.includes('return "FreeTSA 时间戳服务"') &&
    sources.copyrightCard.includes("formatCopyrightDateTime(record.createdAt)") &&
    sources.copyrightCard.includes("formatTimeProofService(record.tsaSource)") &&
    sources.vault.includes("formatCopyrightDateTime(record.createdAt)") &&
    sources.vault.includes("formatTimeProofService(record.tsaSource)"),
  "P0 copyright UI must use stable timestamps and productized time-proof service names",
);

assert(
  !sources.copyrightCard.includes("formatLocalDateTime(record.createdAt)") &&
    !sources.copyrightCard.includes('record.tsaSource?.trim() || "第三方时间戳服务"') &&
    !sources.vault.includes('value: record.tsaSource?.trim() || "第三方时间戳服务"') &&
    !sources.vault.includes(
      'selectedLineageRecord.tsaSource?.trim() || "第三方时间戳服务"',
    ),
  "P0 copyright UI must not fall back to localized timestamps or raw TSA endpoint URLs",
);

for (const [name, source] of Object.entries({
  app: sources.app,
  help: sources.help,
  legal: sources.legal,
  settings: sources.settings,
  proBadge: sources.proBadge,
})) {
  assert(!source.includes("Free 和 Creator"), `${name} must not expose old plan comparison copy`);
  assert(!source.includes("订阅方案"), `${name} must use annual authorization wording`);
}

console.log("Desktop release baseline contract OK");
