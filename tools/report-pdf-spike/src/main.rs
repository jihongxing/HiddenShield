use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use printpdf::{
    Color, Mm, Op, PaintMode, ParsedFont, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions,
    Point, Pt, Rect, Rgb, TextItem,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const PAGE_WIDTH_MM: f32 = 210.0;
const PAGE_HEIGHT_MM: f32 = 297.0;

#[derive(Debug)]
struct Config {
    sample: PathBuf,
    font_sans: PathBuf,
    font_serif: PathBuf,
    output: PathBuf,
    metrics: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageSample {
    report_id: String,
    watermark_uid: String,
    work_title: String,
    work_short: String,
    creator: String,
    exported_at: String,
    created_at: String,
    media_type: String,
    media_spec: String,
    payload: String,
    revision: String,
    original_hash: String,
    protected_hash: String,
    summary_headline: String,
    summary_narrative: String,
    cover_status: String,
    verified: Vec<String>,
    gaps: Vec<String>,
    timeline: Vec<[String; 3]>,
    boundary_verified: Vec<String>,
    boundary_declared: Vec<String>,
    boundary_excluded: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Metrics {
    engine: &'static str,
    implementation: &'static str,
    output_path: String,
    font_load_ms: f64,
    layout_ms: f64,
    serialization_ms: f64,
    generation_ms: f64,
    bytes: u64,
    sha256: String,
    page_count: usize,
    warnings: Vec<String>,
    font_embedding: FontEmbedding,
    signature_extension: SignatureExtension,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FontEmbedding {
    requested_subset: bool,
    embedded_font_file_objects: usize,
    to_unicode_maps: usize,
    subset_font_names: Vec<String>,
    base_fonts: Vec<String>,
    source_fonts: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignatureExtension {
    native_support: bool,
    estimated_cost: &'static str,
    notes: &'static str,
}

#[derive(Clone)]
struct Fonts {
    sans: printpdf::FontId,
    serif: printpdf::FontId,
}

#[derive(Clone, Copy)]
struct Palette;

impl Palette {
    fn navy(self) -> Color {
        rgb(0x17, 0x32, 0x4d)
    }

    fn navy_dark(self) -> Color {
        rgb(0x11, 0x28, 0x3c)
    }

    fn copper(self) -> Color {
        rgb(0x9a, 0x6b, 0x2f)
    }

    fn green(self) -> Color {
        rgb(0x24, 0x6b, 0x4a)
    }

    fn ink(self) -> Color {
        rgb(0x17, 0x20, 0x2a)
    }

    fn muted(self) -> Color {
        rgb(0x66, 0x71, 0x7e)
    }

    fn paper(self) -> Color {
        rgb(0xfb, 0xfa, 0xf7)
    }

    fn rule(self) -> Color {
        rgb(0xd8, 0xd3, 0xc8)
    }

    fn green_soft(self) -> Color {
        rgb(0xe7, 0xf1, 0xeb)
    }

    fn copper_soft(self) -> Color {
        rgb(0xf3, 0xea, 0xdc)
    }

    fn navy_soft(self) -> Color {
        rgb(0xe8, 0xee, 0xf3)
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_args(env::args().skip(1).collect())?;
    let sample: ImageSample = serde_json::from_slice(
        &fs::read(&config.sample).map_err(|error| format!("read sample: {error}"))?,
    )
    .map_err(|error| format!("parse sample: {error}"))?;

    if let Some(parent) = config.output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create output dir: {error}"))?;
    }
    if let Some(parent) = config.metrics.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create metrics dir: {error}"))?;
    }

    let total_started = Instant::now();
    let font_started = Instant::now();
    let sans_bytes =
        fs::read(&config.font_sans).map_err(|error| format!("read sans font: {error}"))?;
    let serif_bytes =
        fs::read(&config.font_serif).map_err(|error| format!("read serif font: {error}"))?;
    let mut font_warnings = Vec::new();
    let sans = ParsedFont::from_bytes(&sans_bytes, 0, &mut font_warnings)
        .ok_or_else(|| "parse Noto Sans SC font failed".to_string())?;
    let serif = ParsedFont::from_bytes(&serif_bytes, 0, &mut font_warnings)
        .ok_or_else(|| "parse Noto Serif SC font failed".to_string())?;
    let font_load_ms = elapsed_ms(font_started);

    let layout_started = Instant::now();
    let mut document = PdfDocument::new("HiddenShield 版权证据技术报告 · Rust Native Spike");
    let fonts = Fonts {
        sans: document.add_font(&sans),
        serif: document.add_font(&serif),
    };
    let palette = Palette;
    let pages = vec![
        cover_page(&sample, &fonts, palette),
        summary_page(&sample, &fonts, palette),
        evidence_chain_page(&sample, &fonts, palette),
        boundary_page(&sample, &fonts, palette),
    ];
    let layout_ms = elapsed_ms(layout_started);

    let serialization_started = Instant::now();
    let save_options = PdfSaveOptions {
        subset_fonts: true,
        optimize: true,
        ..Default::default()
    };
    let mut pdf_warnings = Vec::new();
    let pdf = document
        .with_pages(pages)
        .save(&save_options, &mut pdf_warnings);
    let serialization_ms = elapsed_ms(serialization_started);
    fs::write(&config.output, &pdf).map_err(|error| format!("write PDF: {error}"))?;

    let raw = String::from_utf8_lossy(&pdf);
    let metrics = Metrics {
        engine: "rust_native",
        implementation: "printpdf 0.10.1 manual A4 layout",
        output_path: config.output.to_string_lossy().to_string(),
        font_load_ms,
        layout_ms,
        serialization_ms,
        generation_ms: elapsed_ms(total_started),
        bytes: pdf.len() as u64,
        sha256: format!("{:x}", Sha256::digest(&pdf)),
        page_count: 4,
        warnings: font_warnings
            .iter()
            .map(|warning| format!("{warning:?}"))
            .chain(
                pdf_warnings
                    .iter()
                    .filter(|warning| {
                        !matches!(
                            warning.severity,
                            printpdf::PdfParseErrorSeverity::Info
                        )
                    })
                    .map(|warning| format!("{warning:?}")),
            )
            .collect(),
        font_embedding: inspect_fonts(
            &raw,
            &[
                config.font_sans.to_string_lossy().to_string(),
                config.font_serif.to_string_lossy().to_string(),
            ],
        ),
        signature_extension: SignatureExtension {
            native_support: false,
            estimated_cost: "medium",
            notes: "printpdf exposes the underlying lopdf document for incremental post-processing, so a Rust CMS/PAdES signer can stay in-process. Certificate validation, RFC 3161 timestamps, revocation and long-term validation are still separate work.",
        },
    };

    fs::write(
        &config.metrics,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&metrics)
                .map_err(|error| format!("serialize metrics: {error}"))?
        ),
    )
    .map_err(|error| format!("write metrics: {error}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&metrics)
            .map_err(|error| format!("serialize stdout metrics: {error}"))?
    );
    Ok(())
}

fn cover_page(sample: &ImageSample, fonts: &Fonts, palette: Palette) -> PdfPage {
    let mut ops = Vec::new();
    fill_page(&mut ops, palette.navy());
    fill_rect(&mut ops, 164.0, 0.0, 46.0, 297.0, palette.navy_dark());
    line(
        &mut ops,
        18.0,
        276.0,
        192.0,
        276.0,
        rgb(0x66, 0x7b, 0x8c),
        0.5,
    );
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        18.0,
        272.0,
        7.0,
        "HIDDENSHIELD · COPYRIGHT EVIDENCE",
        rgb(0xe8, 0xee, 0xf3),
    );
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        166.0,
        272.0,
        7.0,
        "RUST NATIVE · R0.1",
        rgb(0xd5, 0xa6, 0x6e),
    );
    fill_rect(&mut ops, 18.0, 246.0, 12.0, 12.0, rgb(0x72, 0xd6, 0xca));
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        20.4,
        249.0,
        10.0,
        "HS",
        palette.navy_dark(),
    );
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        34.0,
        250.0,
        13.0,
        "HIDDENSHIELD",
        palette.paper(),
    );
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        34.0,
        244.5,
        7.0,
        "隐盾 · 数字作品版权保护",
        rgb(0xc8, 0xd3, 0xdb),
    );

    stroke_rect(&mut ops, 18.0, 197.0, 50.0, 10.0, palette.copper(), 0.7);
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        22.0,
        200.2,
        8.0,
        "版权管理与争议处理辅助材料",
        rgb(0xf0, 0xd7, 0xb9),
    );
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        18.0,
        181.0,
        7.0,
        "COPYRIGHT EVIDENCE TECHNICAL REPORT",
        rgb(0xd5, 0xa6, 0x6e),
    );
    text(
        &mut ops,
        fonts,
        FontKind::Serif,
        18.0,
        166.0,
        24.0,
        "HiddenShield 版权证据技术报告",
        palette.paper(),
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Sans,
        18.0,
        149.0,
        10.0,
        7.0,
        48,
        "本报告以版权库记录、媒体摘要、写后读取验证、时间材料及关联收据为基础，呈现可复核的技术事实与当前证据边界。",
        rgb(0xd2, 0xda, 0xdf),
    );
    fill_rect(&mut ops, 18.0, 133.0, 34.0, 0.8, rgb(0xd5, 0xa6, 0x6e));
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        18.0,
        118.0,
        16.0,
        &sample.work_title,
        palette.paper(),
    );

    label_value(
        &mut ops,
        fonts,
        18.0,
        96.0,
        "报告编号",
        &sample.report_id,
        palette,
    );
    label_value(
        &mut ops,
        fonts,
        95.0,
        96.0,
        "版权记录编号",
        &sample.watermark_uid,
        palette,
    );
    label_value(
        &mut ops,
        fonts,
        18.0,
        78.0,
        "权利声明主体",
        &sample.creator,
        palette,
    );
    label_value(
        &mut ops,
        fonts,
        95.0,
        78.0,
        "报告生成时间",
        &sample.exported_at,
        palette,
    );

    stroke_rect(&mut ops, 18.0, 42.0, 25.0, 25.0, rgb(0x8d, 0xd5, 0xac), 0.8);
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        21.0,
        56.0,
        9.0,
        "验证通过",
        rgb(0xdf, 0xf2, 0xe7),
    );
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        22.5,
        49.5,
        6.0,
        "WITH GAPS",
        rgb(0xaf, 0xc8, 0xb8),
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Sans,
        49.0,
        62.0,
        8.0,
        5.2,
        50,
        &sample.cover_status,
        rgb(0xd0, 0xd9, 0xdf),
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Sans,
        18.0,
        25.0,
        6.5,
        4.2,
        78,
        "本报告由 HiddenShield 根据版权库记录自动生成，不构成法律意见、司法鉴定意见、公证文书或诉讼结果承诺。",
        rgb(0xa8, 0xb7, 0xc1),
    );
    footer(&mut ops, fonts, &sample.report_id, 1, true, palette);
    PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops)
}

fn summary_page(sample: &ImageSample, fonts: &Fonts, palette: Palette) -> PdfPage {
    let mut ops = base_page(sample, fonts, "执行摘要", 2, palette);
    section_title(
        &mut ops,
        fonts,
        "01",
        "EXECUTIVE SUMMARY",
        "执行摘要",
        258.0,
        palette,
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Sans,
        39.0,
        239.0,
        8.0,
        5.0,
        76,
        "本页将复杂技术记录压缩为可供创作者、平台申诉人员与法律专业人士快速判断的事实摘要。",
        palette.muted(),
    );
    fill_rect(&mut ops, 18.0, 183.0, 174.0, 44.0, rgb(0xf2, 0xf0, 0xea));
    fill_rect(&mut ops, 18.0, 225.0, 174.0, 1.0, palette.navy());
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        22.0,
        217.0,
        7.0,
        "当前技术结论",
        palette.copper(),
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Serif,
        22.0,
        205.0,
        13.0,
        7.0,
        35,
        &sample.summary_headline,
        palette.navy(),
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Sans,
        22.0,
        190.0,
        6.8,
        4.3,
        62,
        &sample.summary_narrative,
        palette.muted(),
    );
    fill_rect(&mut ops, 153.0, 192.0, 28.0, 28.0, palette.green_soft());
    stroke_rect(&mut ops, 153.0, 192.0, 28.0, 28.0, palette.green(), 1.0);
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        159.0,
        205.0,
        18.0,
        "99",
        palette.green(),
    );
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        159.0,
        198.0,
        6.0,
        "技术完整度",
        palette.green(),
    );

    let original_hash_short = short_hash(&sample.original_hash);
    let facts = [
        ("作品类型", sample.media_type.as_str()),
        ("作品规格", sample.media_spec.as_str()),
        ("记录创建时间", sample.created_at.as_str()),
        ("Payload 协议", sample.payload.as_str()),
        ("版本链", sample.revision.as_str()),
        ("原始 SHA-256", original_hash_short.as_str()),
    ];
    for (index, (label, value)) in facts.iter().enumerate() {
        let column = index % 3;
        let row = index / 3;
        let x = 18.0 + column as f32 * 58.0;
        let y = 154.0 - row as f32 * 25.0;
        stroke_rect(&mut ops, x, y, 58.0, 25.0, palette.rule(), 0.4);
        text(
            &mut ops,
            fonts,
            FontKind::Sans,
            x + 3.0,
            y + 17.5,
            6.5,
            label,
            palette.muted(),
        );
        wrapped_text(
            &mut ops,
            fonts,
            FontKind::Sans,
            x + 3.0,
            y + 10.0,
            8.0,
            4.8,
            18,
            value,
            palette.navy(),
        );
    }

    list_panel(
        &mut ops,
        fonts,
        18.0,
        43.0,
        82.0,
        64.0,
        "系统已验证",
        &sample.verified,
        palette.green(),
        palette,
    );
    list_panel(
        &mut ops,
        fonts,
        110.0,
        43.0,
        82.0,
        64.0,
        "需要接收方关注",
        &sample.gaps,
        palette.copper(),
        palette,
    );
    PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops)
}

fn evidence_chain_page(sample: &ImageSample, fonts: &Fonts, palette: Palette) -> PdfPage {
    let mut ops = base_page(sample, fonts, "证据链与生成过程", 3, palette);
    section_title(
        &mut ops,
        fonts,
        "02",
        "EVIDENCE CHAIN",
        "证据链与生成过程",
        258.0,
        palette,
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Sans,
        39.0,
        239.0,
        8.0,
        5.0,
        76,
        "证据链用于解释各项技术事实如何产生、相互关联，并明确哪些步骤由 HiddenShield、用户或第三方完成。",
        palette.muted(),
    );

    let steps = [
        ("媒体输入", "计算原始 SHA-256"),
        ("水印写入", "V3 / 39 bytes"),
        ("写后读取", "编号与 payload 核对"),
        ("版权库记录", "摘要与版本链"),
        ("网络授时", "TSA 请求材料"),
        ("报告快照", "统一事实模型"),
        ("Manifest", "PDF / JSON 摘要"),
        ("签名校验", "后续 PAdES 阶段"),
    ];
    for (index, (title, detail)) in steps.iter().enumerate() {
        let column = index % 4;
        let row = index / 4;
        let x = 18.0 + column as f32 * 44.5;
        let y = 192.0 - row as f32 * 37.0;
        stroke_rect(&mut ops, x, y, 40.5, 31.0, palette.rule(), 0.45);
        fill_rect(
            &mut ops,
            x + 3.0,
            y + 22.0,
            7.0,
            7.0,
            if index == 4 || index == 7 {
                palette.copper()
            } else {
                palette.green()
            },
        );
        text(
            &mut ops,
            fonts,
            FontKind::Sans,
            x + 4.4,
            y + 24.0,
            6.0,
            &format!("{:02}", index + 1),
            palette.paper(),
        );
        text(
            &mut ops,
            fonts,
            FontKind::Sans,
            x + 3.0,
            y + 15.5,
            8.0,
            title,
            palette.navy(),
        );
        wrapped_text(
            &mut ops,
            fonts,
            FontKind::Sans,
            x + 3.0,
            y + 9.0,
            6.2,
            3.8,
            14,
            detail,
            palette.muted(),
        );
    }

    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        18.0,
        143.0,
        7.0,
        "EVENT TIMELINE",
        palette.copper(),
    );
    for (index, event) in sample.timeline.iter().enumerate() {
        let y = 129.0 - index as f32 * 19.0;
        fill_rect(&mut ops, 18.0, y + 5.0, 4.0, 4.0, palette.navy());
        text(
            &mut ops,
            fonts,
            FontKind::Sans,
            26.0,
            y + 10.0,
            6.0,
            &event[0],
            palette.copper(),
        );
        text(
            &mut ops,
            fonts,
            FontKind::Sans,
            26.0,
            y + 4.0,
            8.0,
            &event[1],
            palette.navy(),
        );
        wrapped_text(
            &mut ops,
            fonts,
            FontKind::Sans,
            83.0,
            y + 8.0,
            6.5,
            4.0,
            34,
            &event[2],
            palette.muted(),
        );
    }

    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        18.0,
        48.0,
        7.0,
        "完整性对象与摘要",
        palette.navy(),
    );
    let rows = [
        ("原始图片", "摘要已记录", short_hash(&sample.original_hash)),
        ("保护副本", "摘要已记录", short_hash(&sample.protected_hash)),
        ("TSA 材料", "token 存在", "source: tsa.example".to_string()),
        ("Registry", "待登记", "pending_registration".to_string()),
    ];
    for (index, row) in rows.iter().enumerate() {
        let y = 39.0 - index as f32 * 7.0;
        line(
            &mut ops,
            18.0,
            y - 1.0,
            192.0,
            y - 1.0,
            palette.rule(),
            0.35,
        );
        text(
            &mut ops,
            fonts,
            FontKind::Sans,
            18.0,
            y + 1.0,
            6.2,
            row.0,
            palette.ink(),
        );
        text(
            &mut ops,
            fonts,
            FontKind::Sans,
            58.0,
            y + 1.0,
            6.2,
            row.1,
            palette.muted(),
        );
        text(
            &mut ops,
            fonts,
            FontKind::Sans,
            105.0,
            y + 1.0,
            5.8,
            &row.2,
            palette.ink(),
        );
    }
    PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops)
}

fn boundary_page(sample: &ImageSample, fonts: &Fonts, palette: Palette) -> PdfPage {
    let mut ops = base_page(sample, fonts, "结论、限制与使用说明", 4, palette);
    section_title(
        &mut ops,
        fonts,
        "03",
        "SCOPE & LIMITATIONS",
        "结论、限制与使用说明",
        258.0,
        palette,
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Sans,
        39.0,
        239.0,
        8.0,
        5.0,
        76,
        "本页以明确分栏避免将系统记录、用户声明和法律判断混为一谈。",
        palette.muted(),
    );

    boundary_column(
        &mut ops,
        fonts,
        18.0,
        96.0,
        54.0,
        128.0,
        "系统已验证",
        &sample.boundary_verified,
        palette.green_soft(),
        palette.green(),
        palette,
    );
    boundary_column(
        &mut ops,
        fonts,
        78.0,
        96.0,
        54.0,
        128.0,
        "用户声明",
        &sample.boundary_declared,
        palette.copper_soft(),
        palette.copper(),
        palette,
    );
    boundary_column(
        &mut ops,
        fonts,
        138.0,
        96.0,
        54.0,
        128.0,
        "本报告不证明",
        &sample.boundary_excluded,
        palette.navy_soft(),
        palette.navy(),
        palette,
    );

    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        18.0,
        81.0,
        8.0,
        "建议用途",
        palette.navy(),
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Sans,
        18.0,
        73.0,
        7.2,
        4.6,
        88,
        "可将本报告用于个人版权归档、平台原创申诉、商务合作材料整理，以及向律师说明技术记录。进入争议处理流程时，应同时保留原始电子数据、保护副本、附件、校验结果和获取过程说明。",
        palette.muted(),
    );
    stroke_rect(&mut ops, 18.0, 36.0, 82.0, 24.0, palette.rule(), 0.5);
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        22.0,
        52.0,
        8.0,
        "补充登记确认",
        palette.navy(),
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Sans,
        22.0,
        44.0,
        6.5,
        4.0,
        36,
        "联网后完成版权编号 registry confirm，并将 receipt 纳入报告附件。",
        palette.muted(),
    );
    stroke_rect(&mut ops, 110.0, 36.0, 82.0, 24.0, palette.rule(), 0.5);
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        114.0,
        52.0,
        8.0,
        "保留原始证据材料",
        palette.navy(),
    );
    wrapped_text(
        &mut ops,
        fonts,
        FontKind::Sans,
        114.0,
        44.0,
        6.5,
        4.0,
        36,
        "原始图片不包含在报告中，应按报告摘要在独立介质中妥善保管。",
        palette.muted(),
    );
    PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops)
}

fn base_page(
    sample: &ImageSample,
    fonts: &Fonts,
    title: &str,
    page_number: usize,
    palette: Palette,
) -> Vec<Op> {
    let mut ops = Vec::new();
    fill_page(&mut ops, palette.paper());
    line(&mut ops, 18.0, 278.0, 192.0, 278.0, palette.rule(), 0.5);
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        18.0,
        282.0,
        6.5,
        &sample.work_short,
        palette.muted(),
    );
    text(
        &mut ops,
        fonts,
        FontKind::Sans,
        158.0,
        282.0,
        6.5,
        title,
        palette.muted(),
    );
    footer(
        &mut ops,
        fonts,
        &sample.report_id,
        page_number,
        false,
        palette,
    );
    ops
}

fn section_title(
    ops: &mut Vec<Op>,
    fonts: &Fonts,
    index: &str,
    kicker: &str,
    title: &str,
    y: f32,
    palette: Palette,
) {
    text(
        ops,
        fonts,
        FontKind::Sans,
        18.0,
        y,
        8.0,
        index,
        palette.copper(),
    );
    text(
        ops,
        fonts,
        FontKind::Sans,
        39.0,
        y + 2.0,
        6.5,
        kicker,
        palette.copper(),
    );
    text(
        ops,
        fonts,
        FontKind::Serif,
        39.0,
        y - 9.0,
        20.0,
        title,
        palette.navy(),
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "PDF layout primitive keeps geometry and palette values explicit at call sites"
)]
fn list_panel(
    ops: &mut Vec<Op>,
    fonts: &Fonts,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    title: &str,
    items: &[String],
    accent: Color,
    palette: Palette,
) {
    fill_rect(ops, x, y, width, height, rgb(0xf3, 0xf1, 0xeb));
    fill_rect(ops, x, y + height - 1.2, width, 1.2, accent);
    text(
        ops,
        fonts,
        FontKind::Sans,
        x + 4.0,
        y + height - 9.0,
        8.0,
        title,
        palette.navy(),
    );
    let mut cursor = y + height - 18.0;
    for item in items.iter().take(4) {
        fill_rect(ops, x + 4.0, cursor + 1.0, 3.0, 3.0, palette.green());
        wrapped_text(
            ops,
            fonts,
            FontKind::Sans,
            x + 10.0,
            cursor + 4.0,
            5.4,
            3.5,
            42,
            item,
            palette.muted(),
        );
        cursor -= 10.5;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "PDF layout primitive keeps geometry and palette values explicit at call sites"
)]
fn boundary_column(
    ops: &mut Vec<Op>,
    fonts: &Fonts,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    title: &str,
    items: &[String],
    background: Color,
    accent: Color,
    palette: Palette,
) {
    fill_rect(ops, x, y, width, height, background);
    fill_rect(ops, x, y + height - 1.5, width, 1.5, accent.clone());
    text(
        ops,
        fonts,
        FontKind::Serif,
        x + 4.0,
        y + height - 12.0,
        11.0,
        title,
        palette.navy(),
    );
    let mut cursor = y + height - 24.0;
    for item in items {
        fill_rect(ops, x + 4.0, cursor + 1.0, 2.5, 2.5, accent.clone());
        cursor = wrapped_text(
            ops,
            fonts,
            FontKind::Sans,
            x + 9.0,
            cursor + 4.0,
            7.0,
            4.5,
            21,
            item,
            palette.ink(),
        ) - 5.0;
    }
}

fn label_value(
    ops: &mut Vec<Op>,
    fonts: &Fonts,
    x: f32,
    y: f32,
    label: &str,
    value: &str,
    _palette: Palette,
) {
    text(
        ops,
        fonts,
        FontKind::Sans,
        x,
        y + 6.0,
        6.0,
        label,
        rgb(0xa9, 0xb6, 0xbf),
    );
    text(
        ops,
        fonts,
        FontKind::Sans,
        x,
        y,
        7.5,
        value,
        rgb(0xf0, 0xf3, 0xf4),
    );
}

fn footer(
    ops: &mut Vec<Op>,
    fonts: &Fonts,
    report_id: &str,
    page_number: usize,
    dark: bool,
    palette: Palette,
) {
    let line_color = if dark {
        rgb(0x5b, 0x70, 0x80)
    } else {
        palette.rule()
    };
    let text_color = if dark {
        rgb(0xb8, 0xc5, 0xcd)
    } else {
        palette.muted()
    };
    line(ops, 18.0, 14.0, 192.0, 14.0, line_color, 0.4);
    text(
        ops,
        fonts,
        FontKind::Sans,
        18.0,
        9.0,
        6.0,
        "技术证据报告",
        if dark {
            rgb(0xd5, 0xa6, 0x6e)
        } else {
            palette.copper()
        },
    );
    text(
        ops,
        fonts,
        FontKind::Sans,
        76.0,
        9.0,
        6.0,
        report_id,
        text_color.clone(),
    );
    text(
        ops,
        fonts,
        FontKind::Sans,
        177.0,
        9.0,
        6.0,
        &format!("{page_number:02} / 04"),
        text_color,
    );
}

#[derive(Clone, Copy)]
enum FontKind {
    Sans,
    Serif,
}

#[expect(
    clippy::too_many_arguments,
    reason = "PDF text primitive keeps typography and position explicit at call sites"
)]
fn text(
    ops: &mut Vec<Op>,
    fonts: &Fonts,
    kind: FontKind,
    x: f32,
    y: f32,
    size: f32,
    value: &str,
    color: Color,
) {
    let font = match kind {
        FontKind::Sans => fonts.sans.clone(),
        FontKind::Serif => fonts.serif.clone(),
    };
    ops.extend([
        Op::StartTextSection,
        Op::SetTextCursor {
            pos: Point::new(Mm(x), Mm(y)),
        },
        Op::SetFillColor { col: color },
        Op::SetFont {
            font: PdfFontHandle::External(font),
            size: Pt(size),
        },
        Op::ShowText {
            items: vec![TextItem::Text(value.to_string())],
        },
        Op::EndTextSection,
    ]);
}

#[expect(
    clippy::too_many_arguments,
    reason = "PDF wrapped-text primitive keeps typography and wrapping explicit at call sites"
)]
fn wrapped_text(
    ops: &mut Vec<Op>,
    fonts: &Fonts,
    kind: FontKind,
    x: f32,
    y: f32,
    size: f32,
    line_height_mm: f32,
    max_units: usize,
    value: &str,
    color: Color,
) -> f32 {
    let mut cursor = y;
    for line_value in wrap_text(value, max_units) {
        text(
            ops,
            fonts,
            kind,
            x,
            cursor,
            size,
            &line_value,
            color.clone(),
        );
        cursor -= line_height_mm;
    }
    cursor
}

fn wrap_text(value: &str, max_units: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line_value = String::new();
    let mut units = 0usize;
    for character in value.chars() {
        let char_units = if character.is_ascii() { 1 } else { 2 };
        if units + char_units > max_units && !line_value.is_empty() {
            lines.push(line_value);
            line_value = String::new();
            units = 0;
        }
        line_value.push(character);
        units += char_units;
    }
    if !line_value.is_empty() {
        lines.push(line_value);
    }
    lines
}

fn fill_page(ops: &mut Vec<Op>, color: Color) {
    fill_rect(ops, 0.0, 0.0, PAGE_WIDTH_MM, PAGE_HEIGHT_MM, color);
}

fn fill_rect(ops: &mut Vec<Op>, x: f32, y: f32, width: f32, height: f32, color: Color) {
    ops.extend([
        Op::SetFillColor { col: color },
        Op::DrawRectangle {
            rectangle: Rect {
                x: Mm(x).into(),
                y: Mm(y).into(),
                width: Mm(width).into(),
                height: Mm(height).into(),
                mode: Some(PaintMode::Fill),
                winding_order: None,
            },
        },
    ]);
}

fn stroke_rect(
    ops: &mut Vec<Op>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
    thickness: f32,
) {
    ops.extend([
        Op::SetOutlineColor { col: color },
        Op::SetOutlineThickness { pt: Pt(thickness) },
        Op::DrawRectangle {
            rectangle: Rect {
                x: Mm(x).into(),
                y: Mm(y).into(),
                width: Mm(width).into(),
                height: Mm(height).into(),
                mode: Some(PaintMode::Stroke),
                winding_order: None,
            },
        },
    ]);
}

fn line(ops: &mut Vec<Op>, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, thickness: f32) {
    ops.extend([
        Op::SetOutlineColor { col: color },
        Op::SetOutlineThickness { pt: Pt(thickness) },
        Op::DrawLine {
            line: printpdf::Line {
                points: vec![
                    printpdf::LinePoint {
                        p: Point::new(Mm(x1), Mm(y1)),
                        bezier: false,
                    },
                    printpdf::LinePoint {
                        p: Point::new(Mm(x2), Mm(y2)),
                        bezier: false,
                    },
                ],
                is_closed: false,
            },
        },
    ]);
}

fn rgb(red: u8, green: u8, blue: u8) -> Color {
    Color::Rgb(Rgb::new(
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
        None,
    ))
}

fn short_hash(value: &str) -> String {
    if value.len() <= 28 {
        return value.to_string();
    }
    format!("{}…{}", &value[..12], &value[value.len() - 12..])
}

fn inspect_fonts(raw: &str, source_fonts: &[String]) -> FontEmbedding {
    let base_fonts = captures(raw, "/BaseFont /");
    FontEmbedding {
        requested_subset: true,
        embedded_font_file_objects: count_all(raw, &["/FontFile ", "/FontFile2 ", "/FontFile3 "]),
        to_unicode_maps: raw.matches("/ToUnicode").count(),
        subset_font_names: base_fonts
            .iter()
            .filter(|name| {
                let bytes = name.as_bytes();
                name.len() > 7 && bytes[..6].iter().all(u8::is_ascii_uppercase) && bytes[6] == b'+'
            })
            .cloned()
            .collect(),
        base_fonts,
        source_fonts: source_fonts.to_vec(),
    }
}

fn captures(raw: &str, prefix: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remainder = raw;
    while let Some(index) = remainder.find(prefix) {
        remainder = &remainder[index + prefix.len()..];
        let value: String = remainder
            .chars()
            .take_while(|character| !character.is_whitespace() && !"/<>()[]".contains(*character))
            .collect();
        if !value.is_empty() && !values.contains(&value) {
            values.push(value);
        }
    }
    values
}

fn count_all(raw: &str, needles: &[&str]) -> usize {
    needles
        .iter()
        .map(|needle| raw.matches(needle).count())
        .sum()
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

impl Config {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        Ok(Self {
            sample: required_path(&args, "--sample")?,
            font_sans: required_path(&args, "--font-sans")?,
            font_serif: required_path(&args, "--font-serif")?,
            output: required_path(&args, "--output")?,
            metrics: required_path(&args, "--metrics")?,
        })
    }
}

fn required_path(args: &[String], name: &str) -> Result<PathBuf, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| Path::new(&pair[1]).to_path_buf())
        .ok_or_else(|| format!("missing required argument {name}"))
}
