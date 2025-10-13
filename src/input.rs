use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::config::Config;
use crate::model::{Source, detect_source, extract_conversations};

pub struct InputBundle {
    pub source: Source,
    pub fingerprint: String,
    pub ndjson_path: PathBuf,
}

pub fn load_input(config: &Config, run_id: &str) -> Result<InputBundle> {
    let input_path = &config.input_path;
    if !input_path.exists() {
        bail!("input path {} does not exist", input_path.display());
    }

    let extension = input_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();

    eprintln!("loading export from {}", input_path.display());

    match extension.as_str() {
        "zip" => load_zip_input(config, run_id),
        "json" => load_json_input(config, run_id, input_path),
        _ => load_json_input(config, run_id, input_path),
    }
}

fn load_json_input(config: &Config, run_id: &str, path: &Path) -> Result<InputBundle> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open input {}", path.display()))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let fingerprint = fingerprint_bytes(&data);
    let json: serde_json::Value = serde_json::from_slice(&data)
        .with_context(|| format!("failed to parse JSON {}", path.display()))?;
    let source = detect_source(&json)?;
    let conversations = extract_conversations(json, source.clone())?;

    let out_dir = ensure_run_dir(config, run_id)?;
    let ndjson_path = out_dir.join("conversations.ndjson");
    let mut ndjson = File::create(&ndjson_path)
        .with_context(|| format!("failed to create {}", ndjson_path.display()))?;

    eprintln!(
        "serializing {} conversation(s) to {}",
        conversations.len(),
        ndjson_path.display()
    );

    let mut count = 0usize;
    for (idx, convo) in conversations.into_iter().enumerate() {
        let line = serde_json::to_vec(&convo)?;
        ndjson.write_all(&line)?;
        ndjson.write_all(b"\n")?;
        if (idx + 1) % 200 == 0 || idx == 0 {
            eprint!("\r  -> {} conversations serialized", idx + 1);
        }
        count = idx + 1;
    }
    eprintln!(
        "\r  -> finished serializing {} conversations        ",
        count
    );

    Ok(InputBundle {
        source,
        fingerprint,
        ndjson_path,
    })
}

fn load_zip_input(config: &Config, run_id: &str) -> Result<InputBundle> {
    let mut file = File::open(&config.input_path)
        .with_context(|| format!("failed to open input {}", config.input_path.display()))?;
    let mut archive = ZipArchive::new(&mut file)
        .with_context(|| format!("failed to open zip {}", config.input_path.display()))?;

    let out_dir = ensure_run_dir(config, run_id)?;
    let ndjson_path = out_dir.join("conversations.ndjson");
    let mut ndjson = File::create(&ndjson_path)
        .with_context(|| format!("failed to create {}", ndjson_path.display()))?;

    let mut hasher = Sha256::new();
    let mut detected_source = None;

    eprintln!(
        "extracting conversations from {} -> {}",
        config.input_path.display(),
        ndjson_path.display()
    );

    let mut total = 0usize;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if !file.is_file() {
            continue;
        }
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let json: serde_json::Value = serde_json::from_slice(&data)?;
        let source = detect_source(&json)?;
        if let Some(existing) = detected_source {
            if existing != source {
                bail!("zip archive contains mixed conversation sources");
            }
        } else {
            detected_source = Some(source.clone());
        }

        let mut convs = extract_conversations(json, source)?;
        convs.sort_by(|a, b| {
            let id_a = a
                .get("uuid")
                .or_else(|| a.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let id_b = b
                .get("uuid")
                .or_else(|| b.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            id_a.cmp(id_b)
        });

        for (idx, convo) in convs.iter().enumerate() {
            let line = serde_json::to_vec(&convo)?;
            hasher.update(&line);
            ndjson.write_all(&line)?;
            ndjson.write_all(b"\n")?;
            if (idx + 1) % 200 == 0 || idx == 0 {
                eprint!("\r  -> {} conversations serialized", idx + 1);
            }
            total += 1;
        }
    }

    eprintln!(
        "\r  -> finished serializing {} conversations        ",
        total
    );

    let fingerprint = format!("sha256:{}", hex::encode(hasher.finalize()));
    let source = detected_source.unwrap_or(Source::Anthropic);

    Ok(InputBundle {
        source,
        fingerprint,
        ndjson_path,
    })
}

fn ensure_run_dir(config: &Config, run_id: &str) -> Result<PathBuf> {
    let primary = config.tmp_dir.join(run_id);
    match fs::create_dir_all(&primary) {
        Ok(_) => Ok(primary),
        Err(err) => {
            let fallback = env::temp_dir().join("floatctl").join(run_id);
            fs::create_dir_all(&fallback).with_context(|| {
                format!("failed to create fallback tmp dir {}", fallback.display())
            })?;
            eprintln!(
                "warning: unable to create tmp dir {} ({}); falling back to {}",
                primary.display(),
                err,
                fallback.display()
            );
            Ok(fallback)
        }
    }
}

fn fingerprint_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("sha256:{}", hex::encode(digest))
}
