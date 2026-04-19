<script setup lang="ts">
import type { VaultRecord } from "../lib/tauri-api";

defineProps<{
  records: VaultRecord[];
}>();

const platformOutputs = [
  { key: "outputDouyin" as const, label: "抖音" },
  { key: "outputBilibili" as const, label: "B站" },
  { key: "outputXhs" as const, label: "小红书" },
];
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
        <div class="timeline__time">{{ record.createdAt }}</div>
        <div class="timeline__content">
          <div class="timeline__title">
            <strong>{{ record.fileName }}</strong>
            <span>{{ record.resolution }}</span>
          </div>
          <div class="timeline__meta">
            <span>UID {{ record.watermarkUid }}</span>
            <span>{{ record.durationSecs }}s</span>
            <span>{{ record.isHdrSource ? "HDR 源" : "SDR 源" }}</span>
          </div>
          <div class="timeline__tags">
            <span
              v-for="p in platformOutputs.filter(o => record[o.key])"
              :key="p.key"
              class="timeline__tag"
            >
              {{ p.label }}
            </span>
          </div>
        </div>
      </article>
    </div>
  </section>
</template>
