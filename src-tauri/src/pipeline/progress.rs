#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::Serialize;

use crate::commands::transcode::Platform;
use crate::encoder::presets;

pub type PlatformPercents = BTreeMap<String, u8>;

#[derive(Debug, Clone, Serialize)]
pub struct StageFrame {
    pub stage: String,
    pub percent: u8,
    pub platform_percents: PlatformPercents,
}

pub fn empty_platform_percents() -> PlatformPercents {
    let mut percents = BTreeMap::new();
    percents.insert("douyin".to_string(), 0);
    percents.insert("bilibili".to_string(), 0);
    percents.insert("xiaohongshu".to_string(), 0);
    percents
}

pub fn bootstrap_stages(is_hdr: bool, platforms: &[Platform]) -> Vec<StageFrame> {
    let mut results = vec![StageFrame {
        stage: "正在分析视频信息...".to_string(),
        percent: 5,
        platform_percents: empty_platform_percents(),
    }];

    if is_hdr {
        results.push(StageFrame {
            stage: "检测到 iPhone HDR 视频，正在优化色彩...".to_string(),
            percent: 8,
            platform_percents: empty_platform_percents(),
        });
    }

    results.push(StageFrame {
        stage: "正在注入版权基因...".to_string(),
        percent: 15,
        platform_percents: empty_platform_percents(),
    });

    results.push(StageFrame {
        stage: "版权保护已激活".to_string(),
        percent: 20,
        platform_percents: empty_platform_percents(),
    });

    let mut mid = empty_platform_percents();
    for platform in platforms {
        let value = match platform {
            Platform::Douyin => 64,
            Platform::Bilibili => 41,
            Platform::Xiaohongshu => 52,
        };
        mid.insert(platform_key(*platform).to_string(), value);
    }

    let mut done = empty_platform_percents();
    for platform in platforms {
        let key = platform_key(*platform).to_string();
        done.insert(key, 100);
    }

    let labels = platforms
        .iter()
        .map(|platform| presets::platform_label(*platform))
        .collect::<Vec<_>>()
        .join(" / ");

    results.push(StageFrame {
        stage: format!("正在生成全平台最优画质... {labels}"),
        percent: 74,
        platform_percents: mid,
    });

    results.push(StageFrame {
        stage: "全部文件已就绪".to_string(),
        percent: 100,
        platform_percents: done,
    });

    results
}

fn platform_key(platform: Platform) -> &'static str {
    match platform {
        Platform::Douyin => "douyin",
        Platform::Bilibili => "bilibili",
        Platform::Xiaohongshu => "xiaohongshu",
    }
}
