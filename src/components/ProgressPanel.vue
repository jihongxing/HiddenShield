<script setup lang="ts">
import { computed } from "vue";
import type { PipelineProgressPayload } from "../lib/tauri-api";

const props = defineProps<{
  busy: boolean;
  summary: string;
  progress: PipelineProgressPayload;
}>();

const emit = defineEmits<{ retry: [] }>();

const isFailed = computed(() => props.progress.stage.startsWith("失败"));
const failureReason = computed(() => {
  if (!isFailed.value) return "";
  return props.progress.stage.replace(/^失败[：:]?\s*/, "");
});

const steps = [
  { title: "读取作品", detail: "读取文件并确认作品信息" },
  { title: "准备版权信息", detail: "准备版权编号与作品声明" },
  { title: "生成保护副本", detail: "写入保护信息并保存副本" },
  { title: "验证保护结果", detail: "回读并确认版权编号" },
  { title: "保存版权记录", detail: "保存记录与存证摘要" },
];

const activeStepIndex = computed(() => {
  const stage = props.progress.stage;
  if (props.progress.percent >= 100) return steps.length;
  if (stage.includes("保存版权记录") || stage.includes("生成存证摘要")) return 4;
  if (stage.includes("回读") || stage.includes("验收")) return 3;
  if (
    stage.includes("写入") ||
    stage.includes("水印") ||
    stage.includes("保护副本") ||
    stage.includes("频域")
  ) {
    return 2;
  }
  if (
    stage.includes("版权载荷") ||
    stage.includes("版权基因") ||
    stage.includes("重写状态") ||
    stage.includes("任务已排队")
  ) {
    return 1;
  }
  return 0;
});

function stepState(index: number): "completed" | "active" | "failed" | "pending" {
  if (props.progress.percent >= 100) return "completed";
  if (isFailed.value && index === activeStepIndex.value) return "failed";
  if (index < activeStepIndex.value) return "completed";
  if (props.busy && index === activeStepIndex.value) return "active";
  return "pending";
}
</script>

<template>
  <section class="panel progress-panel">
    <div class="panel__header">
      <div>
        <h3>处理步骤</h3>
        <p>{{ busy ? progress.stage : isFailed ? "处理失败" : summary }}</p>
      </div>
      <span class="pill">{{ progress.percent >= 100 ? "已完成" : busy ? "处理中" : "待开始" }}</span>
    </div>

    <ol class="progress-steps">
      <li
        v-for="(step, index) in steps"
        :key="step.title"
        class="progress-step"
        :class="`progress-step--${stepState(index)}`"
      >
        <div class="progress-step__marker">
          <span>{{ stepState(index) === "completed" ? "✓" : index + 1 }}</span>
        </div>
        <div class="progress-step__content">
          <strong>{{ step.title }}</strong>
          <span>{{ step.detail }}</span>
        </div>
      </li>
    </ol>

    <!-- Failure details and retry -->
    <div v-if="isFailed" class="progress-panel__failure">
      <p class="progress-panel__failure-reason">{{ failureReason || "未知错误" }}</p>
      <button class="primary-button" type="button" @click="emit('retry')">重试</button>
    </div>
  </section>
</template>
