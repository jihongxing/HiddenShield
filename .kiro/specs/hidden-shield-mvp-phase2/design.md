# 技术设计文档：隐盾 MVP Phase 2 — 体验强化与商业闭环

## 概述

Phase 2 在 Phase 1 核心功能闭环基础上，实现 8 个功能方向：首次自检、结果感知、智能推荐、版权显性化、取证闭环、任务管理、Pro 钩子、信任感。改动集中在前端 UI 层和少量后端新增命令。

参考文档：
- `.kiro/specs/hidden-shield-mvp/design.md` — Phase 1 技术设计
- `.kiro/specs/hidden-shield-mvp-phase2/requirements.md` — Phase 2 需求规格

## 架构变更

### 新增后端命令

| 命令 | 文件 | 输入 | 输出 | 说明 |
|------|------|------|------|------|
| `system_check` | commands/probe.rs | 无 | `Result<SystemCheckResult, String>` | 环境自检 |
| `open_output_dir` | commands/transcode.rs | `path: String` | `Result<(), String>` | 打开系统文件管理器 |

### 新增前端组件

| 组件 | 职责 |
|------|------|
| `SystemStatus.vue` | 环境自检状态卡片 |
| `SourceWarnings.vue` | 素材异常前置提醒 |
| `ResultPage.vue` | 处理完成结果页 + 前后对比 + 存证卡片 |
| `CopyrightCard.vue` | 版权存证卡片（复用于结果页和版权库） |
| `TaskHistory.vue` | 任务历史列表 |
| `ProBadge.vue` | Pro 功能标识组件 |

### 前端视图改造

| 视图 | 改造内容 |
|------|---------|
| `WorkbenchView.vue` | 集成 SystemStatus、SourceWarnings、ResultPage；平台自动推荐；关闭保护 |
| `VaultView.vue` | 集成 CopyrightCard；Pro 导出按钮 |
| `VerifyView.vue` | 增强结果分级展示、关联原记录、取证摘要复制；Pro PDF 按钮 |
| `App.vue` | 侧边栏信任标识；关闭保护逻辑 |

---

## 数据模型

### SystemCheckResult（环境自检结果）

```rust
#[derive(serde::Serialize)]
pub struct SystemCheckResult {
    pub ffmpeg_available: bool,
    pub ffmpeg_version: Option<String>,
    pub gpu_encoder_available: bool,
    pub gpu_encoder_name: Option<String>,
    pub disk_free_mb: u64,
    pub disk_sufficient: bool,
    pub output_dir_writable: bool,
    pub output_dir: String,
}
```

### PipelineCompletePayload（压制完成事件载荷）

```rust
#[derive(serde::Serialize, Clone)]
pub struct PipelineCompletePayload {
    pub pipeline_id: String,
    pub outputs: Vec<OutputFileInfo>,
    pub watermark_uid: String,
    pub process_time_ms: u64,
    pub hw_encoder_used: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct OutputFileInfo {
    pub platform: String,
    pub path: String,
    pub file_size_mb: f64,
    pub resolution: String,
    pub fps: u32,
}
```

### 前端 TypeScript 接口新增

```typescript
export interface SystemCheckResult {
  ffmpegAvailable: boolean;
  ffmpegVersion: string | null;
  gpuEncoderAvailable: boolean;
  gpuEncoderName: string | null;
  diskFreeMb: number;
  diskSufficient: boolean;
  outputDirWritable: boolean;
  outputDir: string;
}

export interface PipelineCompletePayload {
  pipelineId: string;
  outputs: OutputFileInfo[];
  watermarkUid: string;
  processTimeMs: number;
  hwEncoderUsed: string | null;
}

export interface OutputFileInfo {
  platform: string;
  path: string;
  fileSizeMb: number;
  resolution: string;
  fps: number;
}

export interface SourceWarning {
  type: 'info' | 'warning';
  message: string;
}

export interface TaskHistoryItem {
  id: number;
  fileName: string;
  createdAt: string;
  status: 'completed' | 'failed';
  platforms: string[];
  processTimeMs: number | null;
  errorMessage?: string;
}
```

---

## 核心前端逻辑

### 平台推荐算法

```typescript
export function recommendPlatforms(meta: SourceMeta): Platform[] {
  const isPortrait = meta.height > meta.width;
  const isShort = meta.durationSecs < 180;
  const isMedium = meta.durationSecs >= 180 && meta.durationSecs <= 1200;

  if (isPortrait && isShort) return ['douyin', 'xiaohongshu'];
  if (isPortrait && isMedium) return ['douyin', 'xiaohongshu', 'bilibili'];
  if (!isPortrait) return ['bilibili'];
  return ['douyin', 'bilibili'];
}
```

### 策略推荐算法

```typescript
export function recommendStrategy(
  meta: SourceMeta,
  platforms: Platform[],
  hwInfo: HardwareInfo
): TranscodeOptions {
  const needsPortrait = platforms.some(p => p === 'douyin' || p === 'xiaohongshu');
  const isLandscape = meta.width > meta.height;

  return {
    aspectStrategy: (isLandscape && needsPortrait) ? 'letterbox' : 'letterbox',
    encodingMode: hwInfo.preferredEncoder !== 'software' ? 'fast_gpu' : 'high_quality_cpu',
  };
}
```

### 素材警告生成

```typescript
export function generateWarnings(meta: SourceMeta, platforms: Platform[]): SourceWarning[] {
  const warnings: SourceWarning[] = [];

  if (meta.isHdr) {
    warnings.push({ type: 'info', message: '当前素材是 HDR，将自动进行色调映射以确保平台兼容' });
  }

  const isLandscape = meta.width > meta.height;
  const needsPortrait = platforms.some(p => p === 'douyin' || p === 'xiaohongshu');
  if (isLandscape && needsPortrait) {
    warnings.push({ type: 'warning', message: '横屏素材发抖音/小红书展示面积较小，建议考虑智能裁剪' });
  }

  if (meta.fileType === 'video' && meta.durationSecs === 0) {
    warnings.push({ type: 'warning', message: '当前素材无音轨，将跳过音频水印保护' });
  }

  if (meta.durationSecs > 1800) {
    warnings.push({ type: 'warning', message: '素材时长超过 30 分钟，预计处理时间较长' });
  }

  if (meta.fps >= 50) {
    warnings.push({ type: 'info', message: `当前素材帧率 ${meta.fps}fps，B站将保留 60fps，其他平台降为 30fps` });
  }

  return warnings;
}
```

### 取证摘要生成

```typescript
export function buildVerificationSummary(result: VerificationResult, filePath: string): string {
  const now = new Date().toLocaleString('zh-CN');
  const status = result.matched ? '✅ 已命中' : result.confidence >= 0.5 ? '⚠️ 疑似命中' : '❌ 未命中';

  return [
    `【隐盾版权取证摘要】`,
    `检测时间：${now}`,
    `疑似文件：${filePath}`,
    `检测结果：${status}`,
    result.watermarkUid ? `水印 UID：${result.watermarkUid}` : '',
    `置信度：${(result.confidence * 100).toFixed(1)}%`,
    ``,
    `${result.disclaimer}`,
  ].filter(Boolean).join('\n');
}
```

### 存证摘要生成

```typescript
export function buildCopyrightSummary(record: VaultRecord): string {
  const platforms = [
    record.outputDouyin ? '抖音' : '',
    record.outputBilibili ? 'B站' : '',
    record.outputXhs ? '小红书' : '',
  ].filter(Boolean).join('、');

  return [
    `【隐盾版权存证】`,
    `水印 UID：${record.watermarkUid}`,
    `处理时间：${record.createdAt}`,
    `原文件：${record.fileName}`,
    `SHA-256：${record.originalHash}`,
    `输出平台：${platforms}`,
    `分辨率：${record.resolution}`,
    record.hwEncoderUsed ? `编码器：${record.hwEncoderUsed}` : '',
    record.processTimeMs ? `处理耗时：${(record.processTimeMs / 1000).toFixed(1)}s` : '',
  ].filter(Boolean).join('\n');
}
```

---

## 关闭保护机制

使用 Tauri 的 `window.onCloseRequested` 事件：

```typescript
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();
appWindow.onCloseRequested(async (event) => {
  if (hasActivePipeline.value) {
    const confirmed = await confirm('当前有任务正在处理，关闭将中断任务。确定关闭？');
    if (!confirmed) {
      event.preventDefault();
    }
  }
});
```

浏览器模式下使用 `beforeunload` 事件作为 fallback。

---

## Pro 功能钩子设计

MVP 阶段不实际限制功能，仅展示提示和置灰按钮：

| 功能点 | 展示方式 | 触发条件 |
|--------|---------|---------|
| 多平台并行 | 选择 2+ 平台时底部提示 | 选择平台数 > 1 |
| 批量处理 | 侧边栏入口 + Pro 徽章 | 始终展示 |
| 版权库导出 | 按钮置灰 + Pro 标签 | 始终展示 |
| 取证 PDF 报告 | 按钮置灰 + Pro 标签 | 始终展示 |

所有 Pro 提示点击后调用已有的 `trackClick('upgrade_pro_click')` 记录事件。

---

## 错误处理

Phase 2 新增的错误场景：
- `system_check`：不会失败，所有检测项独立执行，失败项标记为 false
- `open_output_dir`：目录不存在时返回错误提示"输出目录不存在，可能已被移动或删除"
- 关闭保护：对话框取消时阻止关闭，确认时正常关闭（不等待任务结束）
- 前端推荐/警告逻辑：纯计算，无异常路径
