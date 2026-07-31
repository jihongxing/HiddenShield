# HiddenShield 中文宣传片

用途：社交媒体横屏投放，同时面向个人创作者、投资人和潜在合作伙伴说明桌面端现有能力与未来规划。

## 输出

- 拟人中文配音主片：`output/promo-video/HiddenShield-宣传片-拟人中文配音-16x9.mp4`
- 拟人中文配音 720p 预览：`output/promo-video/HiddenShield-宣传片-拟人中文配音-预览-720p.mp4`
- 拟人中文配音 30 秒试听：`output/promo-video/HiddenShield-拟人配音试听-30秒.mp4`
- 原系统配音版继续保留在 `output/promo-video/HiddenShield-宣传片-中文-16x9.mp4`
- 分镜图：`output/promo-video/scenes/`
- 拟人配音与镜头时长：`output/promo-video/voice-neural/`、`output/promo-video/scene-timing.json`

## 制作原则

- 当前能力只展示桌面端图片 / 音频保护写入、验证、本地版权库和技术证据报告。
- 云版权库、SDK、API 必须显式标记为“未来规划”。
- 个人作品身份必须显式标记为“终局愿景”。
- 技术证据报告不包装为实名认证、司法确权、发行方数字签名或法律权属结论。
- 演示作品与版权编号均为宣传片视觉素材，不进入正式版权库、报告或同步数据。

## 重新构建

在仓库根目录执行：

```powershell
powershell.exe -ExecutionPolicy Bypass -File docs/promo-video/build-video.ps1
```

首次构建先安装锁定版本的语音依赖：

```powershell
python -m pip install -r docs/promo-video/requirements-voice.txt
```

默认使用温暖、自然的中文神经网络语音 `zh-CN-XiaoxiaoNeural`，语速 `-2%`、音高 `-2Hz`，并由 FFmpeg 生成低音量原创环境音轨。该语音需要联网生成；生成后的逐镜头 MP3 会保存在工程输出目录，后续重新合成视频时可复用。
