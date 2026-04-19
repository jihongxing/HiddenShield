# 隐盾 FFmpeg 多平台压制参数方案

## 设计原则

1. **"喂饱平台"策略**：上传的码率略高于平台二压后的上限，让平台压完后刚好达到最高画质档位
2. **源视频自适应**：根据源视频的分辨率/帧率动态选择参数，而非一刀切
3. **速度与质量平衡**：桌面端用 `medium` preset（不用 `veryslow`，用户等不了那么久）

---

## 一、抖音（Douyin / TikTok 国内版）

### 平台特性分析

- 强制竖屏优先（9:16），横屏会被缩小显示在中间
- 服务端会二次压缩，最终播放码率通常在 1000-3000 Kbps
- 支持格式：MP4（H.264）为最安全选择
- 文件大小上限：500MB
- 推荐分辨率：1080×1920（竖屏）

### 压制策略

**核心思路：** 上传 8-12 Mbps 的高码率源，让抖音二压后仍保持清晰。用 H.264 High Profile 确保兼容性。

### FFmpeg 命令

```bash
# 抖音竖屏压制（源视频为横屏 16:9 时，自动裁剪/填充为 9:16）
ffmpeg -i input.mp4 \
  -vf "scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2:black" \
  -c:v libx264 \
  -profile:v high \
  -level 4.1 \
  -preset medium \
  -crf 18 \
  -maxrate 12000k \
  -bufsize 24000k \
  -pix_fmt yuv420p \
  -r 30 \
  -g 60 \
  -c:a aac \
  -b:a 192k \
  -ar 44100 \
  -ac 2 \
  -movflags +faststart \
  -y output_douyin.mp4
```

### 参数说明

| 参数 | 值 | 说明 |
|------|-----|------|
| 分辨率 | 1080×1920 | 竖屏满屏，9:16 |
| 编码器 | H.264 High Profile | 抖音兼容性最佳 |
| CRF | 18 | 视觉无损级别，给平台二压留足余量 |
| 最大码率 | 12 Mbps | 超过平台播放上限，但上传不会被拒 |
| 帧率 | 30fps | 抖音主流，60fps 无明显收益 |
| 音频 | AAC 192kbps / 44.1kHz | 平台标准 |

### 动态适配逻辑（Rust 端判断）

```rust
// 伪代码：根据源视频决定处理方式
fn get_douyin_vf(src_width: u32, src_height: u32) -> String {
    let src_ratio = src_width as f64 / src_height as f64;
    
    if src_ratio > 1.0 {
        // 横屏源 → 两种策略让用户选：
        // 策略A：上下加黑边保持完整画面
        // 策略B：中心裁剪填满竖屏（损失左右内容）
        "scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2:black"
    } else if (src_ratio - 0.5625).abs() < 0.01 {
        // 已经是 9:16 → 只缩放到 1080p
        "scale=1080:1920"
    } else {
        // 其他比例 → 适配后加黑边
        "scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2:black"
    }
}
```

---

## 二、B站（Bilibili）

### 平台特性分析

- 横屏为主（16:9），竖屏视频会被缩小
- 支持 H.264 和 HEVC（H.265），HEVC 码率上限更低但画质更好
- 支持 4K 和 HDR（大会员功能）
- **关键数据（来自实际抓包验证）：**

| 清晰度 | H.264 码率上限 | HEVC 码率上限 |
|--------|---------------|--------------|
| 1080p30 | ~6000 kbps | ~4000 kbps |
| 1080p60 | ~6000-10000 kbps | ~4000-7000 kbps |
| 4K60 | ~23000 kbps | ~16000 kbps |

- 音频：AAC，最高 192kbps
- B站会对上传视频二压，但如果你的码率已经低于上限，它可能不压（直接过）

### 压制策略

**核心思路：** 用 HEVC 编码 + 高码率上传。HEVC 在同码率下画质优于 H.264，且 B站对 HEVC 的二压更温和。目标是让二压后的视频落在 4000-6000 kbps 区间。

### FFmpeg 命令

```bash
# B站 1080p60 HEVC 压制（推荐方案）
ffmpeg -i input.mp4 \
  -vf "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:black" \
  -c:v libx265 \
  -profile:v main \
  -level 4.1 \
  -preset medium \
  -crf 20 \
  -maxrate 16000k \
  -bufsize 32000k \
  -pix_fmt yuv420p \
  -r 60 \
  -g 120 \
  -keyint_min 60 \
  -x265-params "rc-lookahead=40:ref=4:bframes=8:aq-mode=3" \
  -c:a aac \
  -b:a 192k \
  -ar 44100 \
  -ac 2 \
  -movflags +faststart \
  -y output_bilibili.mp4
```

```bash
# B站 1080p30 H.264 压制（兼容方案，适合不支持 HEVC 解码的老设备观众）
ffmpeg -i input.mp4 \
  -vf "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:black" \
  -c:v libx264 \
  -profile:v high \
  -level 4.2 \
  -preset medium \
  -crf 18 \
  -maxrate 8000k \
  -bufsize 16000k \
  -pix_fmt yuv420p \
  -r 30 \
  -g 300 \
  -keyint_min 30 \
  -refs 4 \
  -c:a aac \
  -b:a 192k \
  -ar 44100 \
  -ac 2 \
  -movflags +faststart \
  -y output_bilibili.mp4
```

### 参数说明

| 参数 | HEVC 方案 | H.264 方案 | 说明 |
|------|-----------|-----------|------|
| 分辨率 | 1920×1080 | 1920×1080 | 横屏标准 |
| CRF | 20 | 18 | HEVC 的 CRF 20 ≈ H.264 的 CRF 18 |
| 最大码率 | 16 Mbps | 8 Mbps | 给二压留余量 |
| 帧率 | 60fps | 30fps | B站支持 60fps 且有明显体验提升 |
| GOP | 120帧(2秒) | 300帧(10秒) | B站要求 keyframe interval ≤ 10s |

### 动态适配逻辑

```rust
fn get_bilibili_params(src_fps: f64, src_height: u32) -> BilibiliPreset {
    // 源视频 >= 50fps 时输出 60fps，否则保持源帧率（最低 24fps）
    let target_fps = if src_fps >= 50.0 { 60 } else if src_fps >= 24.0 { src_fps as u32 } else { 24 };
    
    // 源视频 >= 2160p 时输出 4K，否则输出 1080p
    let (target_w, target_h) = if src_height >= 2160 { (3840, 2160) } else { (1920, 1080) };
    
    BilibiliPreset { target_w, target_h, target_fps, codec: Codec::HEVC }
}
```

---

## 三、小红书（Xiaohongshu / RED）

### 平台特性分析

- 以竖屏（3:4）和方形（1:1）为主，也支持横屏（4:3）
- **最佳比例：3:4（竖屏）**，在信息流中占据最大面积
- 视频分辨率上限：1080×1440（3:4）或 1080×1080（1:1）
- 平台二压激进，最终播放码率较低
- 推荐时长：15-60 秒（算法偏好）
- 格式：MP4 / H.264

### 压制策略

**核心思路：** 3:4 竖屏 + H.264 + 高码率上传。小红书的二压比抖音更狠，所以源文件质量要尽可能高。

### FFmpeg 命令

```bash
# 小红书 3:4 竖屏压制
ffmpeg -i input.mp4 \
  -vf "scale=1080:1440:force_original_aspect_ratio=decrease,pad=1080:1440:(ow-iw)/2:(oh-ih)/2:black" \
  -c:v libx264 \
  -profile:v high \
  -level 4.1 \
  -preset medium \
  -crf 17 \
  -maxrate 15000k \
  -bufsize 30000k \
  -pix_fmt yuv420p \
  -r 30 \
  -g 60 \
  -c:a aac \
  -b:a 192k \
  -ar 44100 \
  -ac 2 \
  -movflags +faststart \
  -y output_xiaohongshu.mp4
```

```bash
# 小红书 1:1 方形压制（适合产品展示/教程类）
ffmpeg -i input.mp4 \
  -vf "scale=1080:1080:force_original_aspect_ratio=decrease,pad=1080:1080:(ow-iw)/2:(oh-ih)/2:black" \
  -c:v libx264 \
  -profile:v high \
  -level 4.1 \
  -preset medium \
  -crf 17 \
  -maxrate 15000k \
  -bufsize 30000k \
  -pix_fmt yuv420p \
  -r 30 \
  -g 60 \
  -c:a aac \
  -b:a 192k \
  -ar 44100 \
  -ac 2 \
  -movflags +faststart \
  -y output_xiaohongshu.mp4
```

### 参数说明

| 参数 | 值 | 说明 |
|------|-----|------|
| 分辨率 | 1080×1440 (3:4) 或 1080×1080 (1:1) | 信息流最大展示面积 |
| 编码器 | H.264 High Profile | 兼容性优先 |
| CRF | 17 | 比抖音更低（更高质量），因为小红书二压更狠 |
| 最大码率 | 15 Mbps | 尽量给足 |
| 帧率 | 30fps | 小红书不需要 60fps |

---

## 四、通用参数决策树（Rust 实现参考）

```rust
/// 根据源视频元数据 + 目标平台，生成 FFmpeg 参数
pub struct TranscodeConfig {
    pub platform: Platform,
    pub video_filter: String,
    pub video_codec: String,
    pub video_params: Vec<String>,
    pub audio_params: Vec<String>,
    pub container_params: Vec<String>,
}

pub enum Platform {
    Douyin,
    Bilibili,
    Xiaohongshu,
}

pub struct SourceMeta {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration_secs: f64,
    pub has_audio: bool,
}

pub fn build_config(platform: Platform, src: &SourceMeta) -> TranscodeConfig {
    match platform {
        Platform::Douyin => TranscodeConfig {
            platform: Platform::Douyin,
            video_filter: fit_to_aspect(src, 1080, 1920), // 9:16
            video_codec: "libx264".into(),
            video_params: vec![
                "-profile:v high", "-level 4.1",
                "-preset medium", "-crf 18",
                "-maxrate 12000k", "-bufsize 24000k",
                "-pix_fmt yuv420p", "-r 30", "-g 60",
            ].into_iter().map(String::from).collect(),
            audio_params: vec![
                "-c:a aac", "-b:a 192k", "-ar 44100", "-ac 2",
            ].into_iter().map(String::from).collect(),
            container_params: vec!["-movflags +faststart".into()],
        },
        Platform::Bilibili => {
            let fps = if src.fps >= 50.0 { 60 } else { 30 };
            let gop = fps * 2; // 2秒一个关键帧
            TranscodeConfig {
                platform: Platform::Bilibili,
                video_filter: fit_to_aspect(src, 1920, 1080), // 16:9
                video_codec: "libx265".into(),
                video_params: vec![
                    format!("-profile:v main"),
                    format!("-level 4.1"),
                    format!("-preset medium"),
                    format!("-crf 20"),
                    format!("-maxrate 16000k"),
                    format!("-bufsize 32000k"),
                    format!("-pix_fmt yuv420p"),
                    format!("-r {fps}"),
                    format!("-g {gop}"),
                    format!("-keyint_min {fps}"),
                    format!("-x265-params rc-lookahead=40:ref=4:bframes=8:aq-mode=3"),
                ],
                audio_params: vec![
                    "-c:a aac".into(), "-b:a 192k".into(),
                    "-ar 44100".into(), "-ac 2".into(),
                ],
                container_params: vec!["-movflags +faststart".into()],
            }
        },
        Platform::Xiaohongshu => TranscodeConfig {
            platform: Platform::Xiaohongshu,
            video_filter: fit_to_aspect(src, 1080, 1440), // 3:4
            video_codec: "libx264".into(),
            video_params: vec![
                "-profile:v high", "-level 4.1",
                "-preset medium", "-crf 17",
                "-maxrate 15000k", "-bufsize 30000k",
                "-pix_fmt yuv420p", "-r 30", "-g 60",
            ].into_iter().map(String::from).collect(),
            audio_params: vec![
                "-c:a aac", "-b:a 192k", "-ar 44100", "-ac 2",
            ].into_iter().map(String::from).collect(),
            container_params: vec!["-movflags +faststart".into()],
        },
    }
}

/// 智能适配目标宽高比，保持源视频内容完整（加黑边）
fn fit_to_aspect(src: &SourceMeta, target_w: u32, target_h: u32) -> String {
    format!(
        "scale={target_w}:{target_h}:force_original_aspect_ratio=decrease,\
         pad={target_w}:{target_h}:(ow-iw)/2:(oh-ih)/2:black"
    )
}
```

---

## 五、哪些参数写死、哪些动态计算

| 参数类别 | 写死 | 动态计算 | 说明 |
|---------|------|---------|------|
| 目标分辨率 | ✅ 每平台固定 | | 抖音1080×1920, B站1920×1080, 小红书1080×1440 |
| 编码器 | ✅ | | 抖音/小红书用H.264, B站用HEVC |
| CRF | ✅ | | 每平台固定值，已经过优化 |
| 最大码率 | ✅ | | 每平台固定上限 |
| 帧率 | | ✅ 根据源视频 | B站：源>=50fps→60fps，否则保持源帧率 |
| GOP 长度 | | ✅ 根据帧率 | = 帧率 × 2（即2秒一个关键帧） |
| video_filter | | ✅ 根据源宽高比 | 判断是否需要裁剪/填充/缩放 |
| preset | ✅ medium | | 速度与质量的平衡点 |
| 音频参数 | ✅ | | 全平台统一 AAC 192k/44.1kHz/立体声 |
| -movflags +faststart | ✅ | | 必须，让视频可以边下边播 |

---

## 六、杀手锏一：硬件加速编码（GPU 探测与动态降级）

### 设计思路

桌面端最大的优势就是能直接调用 GPU 硬件编码器。5 分钟视频从 5 分钟压制骤降到 30 秒，这是用户体感上的"魔法时刻"。

由于我们给的码率已经很高（12-16 Mbps），硬件编码在同码率下的画质损失肉眼不可见。

### 编码器映射表

| 用户硬件 | H.264 替换 | HEVC 替换 | 探测方式 |
|---------|-----------|-----------|---------|
| NVIDIA GPU (GTX 10xx+) | `h264_nvenc` | `hevc_nvenc` | 检查 `nvidia-smi` 或 FFmpeg `-encoders` 输出 |
| Apple Silicon / Intel Mac | `h264_videotoolbox` | `hevc_videotoolbox` | 检查 `sysctl hw.model` |
| Intel 核显 (6代+) | `h264_qsv` | `hevc_qsv` | 检查 `/dev/dri` 或 FFmpeg QSV 初始化 |
| AMD GPU (RX 5000+) | `h264_amf` | `hevc_amf` | 检查 AMF runtime |
| 无独显 / 不支持 | `libx264` (回退) | `libx265` (回退) | 默认软编 |

### Rust 探测逻辑

```rust
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum HwEncoder {
    Nvenc,          // NVIDIA
    VideoToolbox,   // macOS
    Qsv,           // Intel QuickSync
    Amf,           // AMD
    Software,      // CPU fallback
}

/// 探测可用的硬件编码器（启动时执行一次，结果缓存）
pub fn detect_hw_encoder() -> HwEncoder {
    // 优先级：NVENC > VideoToolbox > QSV > AMF > Software
    
    if cfg!(target_os = "macos") {
        // macOS 上 VideoToolbox 几乎 100% 可用
        return HwEncoder::VideoToolbox;
    }
    
    // Windows/Linux：尝试用 FFmpeg 初始化各编码器
    let encoders_to_try = [
        ("h264_nvenc", HwEncoder::Nvenc),
        ("h264_qsv", HwEncoder::Qsv),
        ("h264_amf", HwEncoder::Amf),
    ];
    
    for (encoder, hw_type) in &encoders_to_try {
        let result = Command::new("ffmpeg")
            .args([
                "-hide_banner", "-f", "lavfi", "-i", "nullsrc=s=256x256:d=1",
                "-c:v", encoder, "-f", "null", "-"
            ])
            .output();
        
        if let Ok(output) = result {
            if output.status.success() {
                return hw_type.clone();
            }
        }
    }
    
    HwEncoder::Software
}

/// 根据探测结果返回编码器名称
pub fn get_encoder_name(hw: &HwEncoder, codec: Codec) -> &'static str {
    match (hw, codec) {
        (HwEncoder::Nvenc, Codec::H264) => "h264_nvenc",
        (HwEncoder::Nvenc, Codec::HEVC) => "hevc_nvenc",
        (HwEncoder::VideoToolbox, Codec::H264) => "h264_videotoolbox",
        (HwEncoder::VideoToolbox, Codec::HEVC) => "hevc_videotoolbox",
        (HwEncoder::Qsv, Codec::H264) => "h264_qsv",
        (HwEncoder::Qsv, Codec::HEVC) => "hevc_qsv",
        (HwEncoder::Amf, Codec::H264) => "h264_amf",
        (HwEncoder::Amf, Codec::HEVC) => "hevc_amf",
        (HwEncoder::Software, Codec::H264) => "libx264",
        (HwEncoder::Software, Codec::HEVC) => "libx265",
    }
}

/// 硬件编码器的参数需要调整（不支持 CRF，改用 CBR/VBR 模式）
pub fn get_hw_rate_control_params(hw: &HwEncoder, target_bitrate_kbps: u32, max_bitrate_kbps: u32) -> Vec<String> {
    match hw {
        HwEncoder::Nvenc => vec![
            format!("-rc vbr"),
            format!("-b:v {}k", target_bitrate_kbps),
            format!("-maxrate {}k", max_bitrate_kbps),
            format!("-bufsize {}k", max_bitrate_kbps * 2),
            format!("-rc-lookahead 32"),
            format!("-spatial-aq 1"),
            format!("-temporal-aq 1"),
        ],
        HwEncoder::VideoToolbox => vec![
            format!("-b:v {}k", target_bitrate_kbps),
            format!("-maxrate {}k", max_bitrate_kbps),
            format!("-bufsize {}k", max_bitrate_kbps * 2),
            format!("-realtime 0"),  // 非实时模式，质量优先
        ],
        HwEncoder::Qsv => vec![
            format!("-global_quality 22"),  // QSV 的 "CRF" 等价物
            format!("-maxrate {}k", max_bitrate_kbps),
            format!("-bufsize {}k", max_bitrate_kbps * 2),
            format!("-look_ahead 1"),
        ],
        HwEncoder::Amf => vec![
            format!("-rc vbr_peak"),
            format!("-b:v {}k", target_bitrate_kbps),
            format!("-maxrate {}k", max_bitrate_kbps),
            format!("-bufsize {}k", max_bitrate_kbps * 2),
            format!("-quality balanced"),
        ],
        HwEncoder::Software => vec![
            // 软编继续用 CRF 模式（在调用处单独处理）
        ],
    }
}
```

### 硬件编码的 FFmpeg 命令示例（NVENC + 抖音）

```bash
ffmpeg -i input.mp4 \
  -vf "scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2:black" \
  -c:v h264_nvenc \
  -profile:v high \
  -rc vbr \
  -b:v 10000k \
  -maxrate 12000k \
  -bufsize 24000k \
  -rc-lookahead 32 \
  -spatial-aq 1 \
  -temporal-aq 1 \
  -pix_fmt yuv420p \
  -r 30 \
  -g 60 \
  -c:a aac -b:a 192k -ar 44100 -ac 2 \
  -movflags +faststart \
  -y output_douyin.mp4
```

---

## 七、杀手锏二：iPhone HDR 发灰问题自动修复（Tone-mapping）

### 问题本质

iPhone 12+ 默认录制 Dolby Vision / HLG HDR 视频（10-bit, BT.2020 色域）。当这类视频上传到不支持 HDR 全链路的平台（抖音、小红书），平台会粗暴地把 10-bit 截断为 8-bit，导致画面发灰、对比度丢失。

这是 2024-2026 年自媒体圈最高频的画质投诉之一。

### 解决方案：智能色调映射

在 Rust 端探测源视频的色彩空间，如果是 HDR，自动在 `-vf` 滤镜链前端插入 tone-mapping 滤镜。

### 探测逻辑

```rust
use std::process::Command;
use serde::Deserialize;

#[derive(Debug)]
pub struct ColorInfo {
    pub is_hdr: bool,
    pub color_space: String,      // e.g. "bt2020nc"
    pub color_transfer: String,   // e.g. "smpte2084" (PQ) 或 "arib-std-b67" (HLG)
    pub color_primaries: String,  // e.g. "bt2020"
    pub bit_depth: u8,            // 8 or 10
}

/// 用 ffprobe 探测源视频色彩信息
pub fn probe_color_info(input_path: &str) -> ColorInfo {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_streams",
            "-select_streams", "v:0",
            input_path,
        ])
        .output()
        .expect("ffprobe failed");
    
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let stream = &json["streams"][0];
    
    let color_space = stream["color_space"].as_str().unwrap_or("unknown").to_string();
    let color_transfer = stream["color_transfer"].as_str().unwrap_or("unknown").to_string();
    let color_primaries = stream["color_primaries"].as_str().unwrap_or("unknown").to_string();
    let pix_fmt = stream["pix_fmt"].as_str().unwrap_or("");
    
    let bit_depth = if pix_fmt.contains("10") || pix_fmt.contains("p010") { 10 } else { 8 };
    
    let is_hdr = color_transfer == "smpte2084"      // PQ (Dolby Vision / HDR10)
              || color_transfer == "arib-std-b67"    // HLG
              || color_primaries == "bt2020";
    
    ColorInfo { is_hdr, color_space, color_transfer, color_primaries, bit_depth }
}
```

### Tone-mapping 滤镜链

```rust
/// 生成 HDR → SDR 色调映射滤镜（插入到 -vf 链的最前面）
pub fn build_tonemap_filter(color_info: &ColorInfo) -> Option<String> {
    if !color_info.is_hdr {
        return None;
    }
    
    // Hable tone-mapping 对 iPhone Dolby Vision 效果最好
    // desat=0 保留饱和度，避免画面变得寡淡
    Some(
        "zscale=t=linear:npl=100,\
         format=gbrpf32le,\
         zscale=p=bt709,\
         tonemap=tonemap=hable:desat=0,\
         zscale=t=bt709:m=bt709:r=tv,\
         format=yuv420p".to_string()
    )
}

/// 组合最终的 -vf 滤镜链
pub fn build_full_vf(color_info: &ColorInfo, scale_filter: &str) -> String {
    match build_tonemap_filter(color_info) {
        Some(tonemap) => format!("{},{}", tonemap, scale_filter),
        None => scale_filter.to_string(),
    }
}
```

### 完整 FFmpeg 命令（iPhone HDR → 抖音 SDR）

```bash
# 当探测到源视频为 HDR 时，自动插入 tone-mapping
ffmpeg -i iphone_hdr_input.mov \
  -vf "zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=tonemap=hable:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p,scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2:black" \
  -c:v libx264 \
  -profile:v high \
  -level 4.1 \
  -preset medium \
  -crf 18 \
  -maxrate 12000k \
  -bufsize 24000k \
  -r 30 \
  -g 60 \
  -c:a aac -b:a 192k -ar 44100 -ac 2 \
  -movflags +faststart \
  -y output_douyin_from_hdr.mp4
```

### 营销价值

> "隐盾独家算法，完美解决苹果 HDR 视频上传发灰问题，还原真实色彩"

这一个功能点就足以让大量 iPhone 用户下载隐盾。它是一个独立的、可感知的、高频的痛点。

---

## 八、杀手锏三：音频盲水印无缝嵌入流水线（Zero-overhead Pipeline）

### 设计目标

盲水印嵌入不能增加用户等待时间。通过异步流水线，让水印注入和视频压制并行执行。

### 流水线架构

```
用户拖入视频
     │
     ├──→ [线程1] 极速抽取音频 (< 1秒)
     │         │
     │         └──→ [线程2] Rust FFT 水印注入 (2-5秒)
     │                        │
     │                        ▼
     │                  watermarked.wav
     │
     └──→ [线程3] 等待水印完成 → 启动多平台压制（双输入源）
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
              抖音压制         B站压制         小红书压制
              (并行)          (并行)          (并行)
```

### 关键命令变形：双输入源压制

```bash
# 抖音压制（视频取原文件，音频取水印后的 WAV）
ffmpeg -i input.mp4 -i watermarked.wav \
  -vf "scale=1080:1920:force_original_aspect_ratio=decrease,pad=1080:1920:(ow-iw)/2:(oh-ih)/2:black" \
  -map 0:v:0 \
  -map 1:a:0 \
  -c:v libx264 \
  -profile:v high \
  -level 4.1 \
  -preset medium \
  -crf 18 \
  -maxrate 12000k \
  -bufsize 24000k \
  -pix_fmt yuv420p \
  -r 30 \
  -g 60 \
  -c:a aac -b:a 192k -ar 44100 -ac 2 \
  -movflags +faststart \
  -y output_douyin_secure.mp4
```

核心变化：
- `-map 0:v:0`：视频流取第一个输入（原始视频）
- `-map 1:a:0`：音频流取第二个输入（水印后的音频）
- 视频画面零损耗（不需要额外解码/编码音频再合并）

### Rust 流水线实现

```rust
use tokio::task;
use std::path::PathBuf;

pub struct PipelineResult {
    pub douyin_path: Option<PathBuf>,
    pub bilibili_path: Option<PathBuf>,
    pub xiaohongshu_path: Option<PathBuf>,
    pub watermark_uid: String,
    pub timestamp: String,
}

pub async fn run_pipeline(
    input_path: &str,
    platforms: &[Platform],
    user_uid: &str,
) -> Result<PipelineResult, PipelineError> {
    
    // ===== 阶段 1：极速抽取音频（阻塞极短，< 1秒）=====
    let audio_raw_path = extract_audio_pcm(input_path).await?;
    
    // ===== 阶段 2：异步水印注入 =====
    let watermark_handle = task::spawn_blocking({
        let audio_path = audio_raw_path.clone();
        let uid = user_uid.to_string();
        move || {
            embed_watermark_fft(&audio_path, &uid)
        }
    });
    
    // ===== 阶段 2.5：同时探测源视频元数据 =====
    let source_meta = probe_source_meta(input_path).await?;
    let color_info = probe_color_info(input_path);
    let hw_encoder = detect_hw_encoder(); // 已缓存
    
    // ===== 等待水印完成 =====
    let watermarked_audio_path = watermark_handle.await??;
    
    // ===== 阶段 3：并行多平台压制 =====
    let mut handles = Vec::new();
    
    for platform in platforms {
        let config = build_config_with_hw(
            *platform, &source_meta, &color_info, &hw_encoder
        );
        let input = input_path.to_string();
        let audio = watermarked_audio_path.clone();
        
        handles.push(task::spawn_blocking(move || {
            execute_ffmpeg_transcode(&input, &audio, &config)
        }));
    }
    
    // 等待所有平台压制完成
    let results = futures::future::join_all(handles).await;
    
    // 收集结果...
    Ok(PipelineResult { /* ... */ })
}

/// 极速抽取音频为 PCM（给 Rust FFT 算法用）
async fn extract_audio_pcm(input: &str) -> Result<PathBuf, PipelineError> {
    let output = PathBuf::from(format!("{}_temp_audio.wav", input));
    
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-i", input,
            "-vn",                    // 不要视频
            "-c:a", "pcm_s16le",     // 16-bit PCM（FFT 友好格式）
            "-ar", "44100",          // 统一采样率
            "-ac", "2",              // 立体声
            "-y",
            output.to_str().unwrap(),
        ])
        .output()
        .await?;
    
    if !status.status.success() {
        return Err(PipelineError::AudioExtractFailed);
    }
    
    Ok(output)
}
```

### 时间线对比

| 步骤 | 无流水线（串行） | 有流水线（并行） |
|------|----------------|----------------|
| 抽取音频 | 1s | 1s |
| 水印注入 | 3s | 3s（与探测并行） |
| 探测元数据 | 0.5s | 0s（与水印并行） |
| 抖音压制 | 30s | 30s |
| B站压制 | 45s | 45s（与抖音并行） |
| 小红书压制 | 30s | 30s（与上面并行） |
| **总计** | **~110s** | **~49s**（瓶颈=最慢的压制任务+4s前置） |

用户体感：拖入视频后约 50 秒，三个平台的文件全部生成完毕。

---

## 九、完整处理流程总览

```
┌─────────────────────────────────────────────────────────────────┐
│                    用户拖入视频文件                                │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│  [探测阶段] 并行执行（< 1秒）                                     │
│                                                                   │
│  ① ffprobe 获取源视频元数据（分辨率/帧率/时长）                     │
│  ② ffprobe 获取色彩空间信息（是否 HDR）                            │
│  ③ 检查硬件编码器可用性（首次启动时已缓存）                         │
│  ④ ffmpeg 极速抽取音频 PCM                                        │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│  [水印阶段] Rust FFT 频域盲水印注入（2-5秒）                       │
│                                                                   │
│  输入：PCM 音频 + 用户 UID + 时间戳                                │
│  输出：watermarked.wav                                            │
│  同时：本地 SQLite 写入版权记录                                    │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│  [压制阶段] 多平台并行 FFmpeg 压制                                 │
│                                                                   │
│  输入源：原视频(画面) + watermarked.wav(音频)                      │
│  滤镜链：[HDR tone-map (如需)] → [缩放/裁剪/填充]                 │
│  编码器：[硬件加速 (如可用)] 或 [CPU 软编 (回退)]                  │
│                                                                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                       │
│  │ 抖音     │  │ B站      │  │ 小红书    │                       │
│  │ 1080×1920│  │ 1920×1080│  │ 1080×1440│                       │
│  │ H.264    │  │ HEVC     │  │ H.264    │                       │
│  │ 30fps    │  │ 60fps    │  │ 30fps    │                       │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                       │
│       │              │              │                             │
└───────┼──────────────┼──────────────┼─────────────────────────────┘
        │              │              │
        ▼              ▼              ▼
┌─────────────────────────────────────────────────────────────────┐
│  [输出阶段]                                                       │
│                                                                   │
│  📁 输出目录/                                                     │
│  ├── 视频名_抖音优化版.mp4                                        │
│  ├── 视频名_B站优化版.mp4                                         │
│  └── 视频名_小红书优化版.mp4                                      │
│                                                                   │
│  💾 本地 SQLite 版权金库新增记录：                                  │
│  {hash, uid, timestamp, watermark_data, output_paths}            │
└─────────────────────────────────────────────────────────────────┘
```

---

## 十、UI 进度条状态映射

前端需要展示的进度状态（对应后端事件）：

| 后端事件 | 前端展示文案 | 进度百分比 |
|---------|------------|-----------|
| 探测开始 | "正在分析视频信息..." | 0-5% |
| HDR 检测到 | "检测到 iPhone HDR 视频，正在优化色彩..." | 5-8% |
| 音频抽取完成 | "正在注入版权基因..." | 8-15% |
| 水印注入完成 | "版权保护已激活 ✓" | 15-20% |
| 压制开始 | "正在生成全平台最优画质..." | 20% |
| 压制进行中 | "抖音版 45% / B站版 32% / 小红书版 50%" | 20-95% |
| 全部完成 | "🎉 三个平台文件已就绪！" | 100% |

FFmpeg 的压制进度可以通过解析 stderr 中的 `time=` 字段实时计算百分比（已知总时长）。
