<script setup lang="ts">
import { formatLocalDateTime, type VaultRecord } from "../lib/tauri-api";

defineProps<{
  records: VaultRecord[];
}>();

</script>

<template>
  <section class="panel">
    <div class="panel__header">
      <div>
        <h3>版权时间线</h3>
        <p>后续这里会接 SQLite 查询、筛选和导出报告能力。</p>
      </div>
      <span class="pill">{{ records.length }} 条</span>
    </div>

    <div class="timeline">
      <article v-for="record in records" :key="record.id" class="timeline__item">
        <div class="timeline__time">{{ formatLocalDateTime(record.createdAt) }}</div>
        <div class="timeline__content">
          <div class="timeline__title">
            <strong>{{ record.fileName }}</strong>
            <span>{{ record.resolution }}</span>
          </div>
          <div class="timeline__meta">
            <span>版权编号 {{ record.watermarkUid }}</span>
            <span v-if="record.revision > 1">第 {{ record.revision }} 次写入</span>
            <span v-if="record.parentWatermarkUid">有上一版</span>
            <span>{{ record.durationSecs }}s</span>
            <span>{{ record.isHdrSource ? "HDR 源" : "SDR 源" }}</span>
          </div>
          <div class="timeline__tags">
            <span v-if="record.protectedCopyName || record.protectedCopyHash" class="timeline__tag">保护副本</span>
            <span class="timeline__tag">最小必要变更</span>
          </div>
        </div>
      </article>
    </div>
  </section>
</template>
