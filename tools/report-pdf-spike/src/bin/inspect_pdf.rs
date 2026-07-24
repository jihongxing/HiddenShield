use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use lopdf::{Document, Object};
use serde::Serialize;

#[derive(Debug)]
struct Config {
    input: PathBuf,
    output: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PdfInspection {
    input_path: String,
    page_count: usize,
    font_dictionaries: usize,
    type3_font_dictionaries: usize,
    type0_font_dictionaries: usize,
    embedded_font_file_objects: usize,
    to_unicode_maps: usize,
    subset_font_names: Vec<String>,
    base_fonts: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_args(env::args().skip(1).collect())?;
    let document = Document::load(&config.input)
        .map_err(|error| format!("load PDF {}: {error}", config.input.display()))?;

    let mut font_dictionaries = 0usize;
    let mut type3_font_dictionaries = 0usize;
    let mut type0_font_dictionaries = 0usize;
    let mut embedded_font_file_objects = 0usize;
    let mut to_unicode_maps = 0usize;
    let mut base_fonts = BTreeSet::new();

    for object in document.objects.values() {
        let dictionary = match object {
            Object::Dictionary(dictionary) => dictionary,
            Object::Stream(stream) => &stream.dict,
            _ => continue,
        };

        if dictionary
            .get(b"Type")
            .ok()
            .and_then(|value| value.as_name().ok())
            == Some(b"Font".as_slice())
        {
            font_dictionaries += 1;
            match dictionary
                .get(b"Subtype")
                .ok()
                .and_then(|value| value.as_name().ok())
            {
                Some(b"Type3") => type3_font_dictionaries += 1,
                Some(b"Type0") => type0_font_dictionaries += 1,
                _ => {}
            }
        }

        if dictionary.has(b"FontFile") {
            embedded_font_file_objects += 1;
        }
        if dictionary.has(b"FontFile2") {
            embedded_font_file_objects += 1;
        }
        if dictionary.has(b"FontFile3") {
            embedded_font_file_objects += 1;
        }
        if dictionary.has(b"ToUnicode") {
            to_unicode_maps += 1;
        }
        if let Ok(value) = dictionary.get(b"BaseFont") {
            if let Ok(name) = value.as_name() {
                base_fonts.insert(String::from_utf8_lossy(name).to_string());
            }
        }
        if let Ok(value) = dictionary.get(b"FontName") {
            if let Ok(name) = value.as_name() {
                base_fonts.insert(String::from_utf8_lossy(name).to_string());
            }
        }
    }

    let subset_font_names = base_fonts
        .iter()
        .filter(|name| is_subset_name(name))
        .cloned()
        .collect();
    let inspection = PdfInspection {
        input_path: config.input.to_string_lossy().to_string(),
        page_count: document.get_pages().len(),
        font_dictionaries,
        type3_font_dictionaries,
        type0_font_dictionaries,
        embedded_font_file_objects,
        to_unicode_maps,
        subset_font_names,
        base_fonts: base_fonts.into_iter().collect(),
    };
    if let Some(parent) = config.output.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create output dir: {error}"))?;
    }
    fs::write(
        &config.output,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&inspection)
                .map_err(|error| format!("serialize inspection: {error}"))?
        ),
    )
    .map_err(|error| format!("write inspection: {error}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&inspection)
            .map_err(|error| format!("serialize stdout: {error}"))?
    );
    Ok(())
}

fn is_subset_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    name.len() > 7 && bytes[..6].iter().all(u8::is_ascii_uppercase) && bytes[6] == b'+'
}

impl Config {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        Ok(Self {
            input: required_path(&args, "--input")?,
            output: required_path(&args, "--output")?,
        })
    }
}

fn required_path(args: &[String], name: &str) -> Result<PathBuf, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| Path::new(&pair[1]).to_path_buf())
        .ok_or_else(|| format!("missing required argument {name}"))
}
