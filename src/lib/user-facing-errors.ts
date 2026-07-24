export function userFacingErrorMessage(error: unknown, context = "操作"): string {
  const raw = String((error as { message?: unknown })?.message ?? error ?? "").trim();
  const lower = raw.toLowerCase();
  const formalWatermarkUidPattern = /HS-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}-[A-F0-9]{8}/;

  if (!raw) return `${context}失败，请稍后重试。`;
  if (raw.includes("offline_license_unknown_key")) {
    return `${context}失败：当前安装版本未内置该年度授权公钥，请更新到最新 HiddenShield 安装包后重试。`;
  }
  if (
    lower.includes("failed to fetch") ||
    lower.includes("clientexception") ||
    lower.includes("networkerror") ||
    lower.includes("connection refused") ||
    lower.includes("timed out") ||
    lower.includes("timeout")
  ) {
    return `${context}失败：暂时无法连接服务，请确认后端服务已启动，或稍后重试。`;
  }
  if (raw.includes("HTTP 401") || lower.includes("unauthorized")) {
    return `${context}失败：登录状态已失效，请重新登录后再试。`;
  }
  if (raw.includes("HTTP 403") || lower.includes("forbidden")) {
    return `${context}失败：当前账户没有执行该操作的权限，请确认账户、工作区和订阅状态。`;
  }
  if (raw.includes("HTTP 408") || raw.includes("HTTP 429") || /HTTP 5\d\d/.test(raw)) {
    return `${context}失败：服务暂时不可用，请稍后重试。`;
  }
  if (raw.includes("wechat_pay_not_configured")) {
    return "支付通道尚未完成配置，当前可先联系开通。";
  }
  if (lower.includes("[already_watermarked]")) {
    const uid = raw.match(formalWatermarkUidPattern)?.[0];
    return uid
      ? `${context}失败：素材中已经存在 HiddenShield 水印，版权记录 ${uid}。如需生成新版，请开启“这是已有作品的新版”。`
      : `${context}失败：素材中已经存在 HiddenShield 水印。如需生成新版，请开启“这是已有作品的新版”。`;
  }
  if (lower.includes("[missing_creator_identity]")) {
    return `${context}失败：请先完成创作者身份设置，再生成保护副本。`;
  }
  if (lower.includes("[embed_failed]")) {
    return `${context}失败：保护副本未生成。请确认文件可读取后重试；如果持续失败，请复制诊断信息反馈。`;
  }
  if (lower.includes("watermark already exists in source media")) {
    const uid = raw.match(formalWatermarkUidPattern)?.[0];
    return uid
      ? `${context}失败：素材中已经存在 HiddenShield 水印，版权记录 ${uid}。如需生成新版，请开启“这是已有作品的新版”。`
      : `${context}失败：素材中已经存在 HiddenShield 水印。如需生成新版，请开启“这是已有作品的新版”。`;
  }
  if (raw.includes("Watermark embedding failed")) {
    return `${context}失败：保护副本未生成。请确认文件可读取后重试；如果持续失败，请复制诊断信息反馈。`;
  }
  if (raw.includes("正式报告") || raw.includes("Creator") || raw.includes("订阅")) {
    return raw;
  }
  return `${context}失败：系统没有完成本次请求，请重试；如果持续失败，请复制诊断信息反馈。`;
}

export function watermarkErrorUserMessage(error: unknown, context = "写入"): string {
  return userFacingErrorMessage(error, context);
}
