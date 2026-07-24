import type { EntitlementState } from "./tauri-api";

export function planLabel(entitlementState: EntitlementState | null): string {
  const active = entitlementState &&
    ["trial", "active", "grace"].includes(entitlementState.status) &&
    entitlementState.features?.batch_processing === true;
  return active ? "图片 / 音频年费" : "未付费";
}
