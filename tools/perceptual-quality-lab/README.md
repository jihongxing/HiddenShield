# HiddenShield 感知质量实验室

这是一个与 HiddenShield 主程序完全分离的 Windows 本地工具，用于比较图片 / 音频原始素材与水印注入后素材，并执行单人 ABX 盲测。

## 能力

- 图片：同步并排、分割、闪烁、`1× / 4× / 16×` 差异热力图。
- 图片指标：PSNR、SSIM、MAE、P95 差异、最大通道差和变化像素率。
- 音频：FFmpeg 解码、`±250 ms` 有限偏移对齐、同步波形和 10 秒试听片段。
- 音频指标：SNR、分段 SNR、当前 gate 口径 LUFS / 峰值差、clipping、静音噪声底和频带能量。
- ABX：10 轮快速筛查或 20 轮正式单人测试，平衡 X 身份并计算单侧二项分布显著性。

本工具不生成水印、不读取版权库、不联网、不保存历史。结果只能说明当前素材、设备和测试环境下是否观察到稳定差异，不能证明“绝对无感”或“零影响”。

## 环境

- Windows 10/11。
- Node.js 与 npm。
- Rust `1.77.2` 或更高兼容工具链。
- `ffmpeg` 和 `ffprobe` 可从 `PATH` 访问。

也可以通过以下环境变量指定可执行文件：

```powershell
$env:HIDDENSHIELD_FFMPEG_PATH = "C:\path\to\ffmpeg.exe"
$env:HIDDENSHIELD_FFPROBE_PATH = "C:\path\to\ffprobe.exe"
```

## 运行

```powershell
cd tools/perceptual-quality-lab
npm install
npm run tauri:dev
```

## 验证

```powershell
npm run test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml
```

如工作盘空间不足，可将 Rust 构建缓存放到其他磁盘：

```powershell
$env:CARGO_TARGET_DIR = "E:\codex-build\HiddenShield-perceptual-quality-lab"
```
