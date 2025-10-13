use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::NaiveDate;
use home::home_dir;
use serde::Deserialize;

use crate::cli::{Cli, CliFormat, DateFrom, SourceSelector};
use crate::util::{DEFAULT_CONFIG_PATH, DEFAULT_STATE_DIR, DEFAULT_TMP_DIR, expand_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutputFormat {
    Md,
    Json,
}

impl OutputFormat {
    pub fn from_cli(value: CliFormat) -> Self {
        match value {
            CliFormat::Md => Self::Md,
            CliFormat::Json => Self::Json,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilenameStrategy {
    Title,
    Id,
    FirstHumanLine,
}

impl FilenameStrategy {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "title" => Some(Self::Title),
            "id" => Some(Self::Id),
            "first-human-line" => Some(Self::FirstHumanLine),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FiltersConfig {
    pub since: Option<NaiveDate>,
    pub until: Option<NaiveDate>,
}

impl Default for FiltersConfig {
    fn default() -> Self {
        Self {
            since: None,
            until: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub input_path: PathBuf,
    pub output_dir: PathBuf,
    pub formats: Vec<OutputFormat>,
    pub timezone: Option<String>,
    pub date_from: DateFrom,
    pub dedupe: bool,
    #[allow(dead_code)]
    pub copy_assets: bool,
    pub pretty_json_indent: usize,
    pub filename_from: FilenameStrategy,
    pub filters: FiltersConfig,
    pub state_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub source: SourceSelector,
    #[allow(dead_code)]
    pub config_path: PathBuf,
}

impl Config {
    pub fn load(provided: Option<&Path>) -> Result<Self> {
        let defaults = RawConfig::default();
        let mut resolved_path = DEFAULT_CONFIG_PATH.clone();
        let loaded = if let Some(explicit) = provided {
            let expanded = expand_path(explicit).context("failed to resolve config path")?;
            resolved_path = expanded.clone();
            if expanded.exists() {
                Some(load_raw_config(&expanded)?)
            } else {
                return Err(anyhow::anyhow!(format!(
                    "config file {} does not exist",
                    expanded.display()
                )));
            }
        } else if DEFAULT_CONFIG_PATH.exists() {
            Some(load_raw_config(&DEFAULT_CONFIG_PATH)?)
        } else {
            find_fallback_config()
                .map(|path| {
                    resolved_path = path.clone();
                    load_raw_config(&path)
                })
                .transpose()?
        };

        let merged = defaults.merge(loaded.unwrap_or_default());
        let formats = merged.resolve_formats()?;

        let output_dir = merged
            .out_dir
            .as_ref()
            .map(|p| expand_path(Path::new(p)))
            .transpose()?
            .unwrap_or_else(|| PathBuf::from("conversations"));

        let timezone = merged.tz.clone();
        let date_from = merged
            .date_from
            .as_deref()
            .and_then(|v| match v {
                "utc" => Some(DateFrom::Utc),
                "local" => Some(DateFrom::Local),
                _ => None,
            })
            .unwrap_or(DateFrom::Utc);

        let dedupe = merged.dedupe.unwrap_or(true);
        let copy_assets = merged.copy_assets.unwrap_or(false);
        let pretty_json_indent = merged.pretty_json_indent.unwrap_or(2usize);

        let filename_from = merged
            .filename_from
            .as_deref()
            .and_then(FilenameStrategy::from_str)
            .unwrap_or(FilenameStrategy::Title);

        let filters = FiltersConfig {
            since: parse_optional_date(merged.filters.since.as_deref())
                .context("invalid 'filters.since' date in config")?,
            until: parse_optional_date(merged.filters.until.as_deref())
                .context("invalid 'filters.until' date in config")?,
        };

        let state_dir = merged
            .state
            .dir
            .as_ref()
            .map(|p| expand_path(Path::new(p)))
            .transpose()?
            .unwrap_or_else(|| DEFAULT_STATE_DIR.join("conv_split"));

        let tmp_dir = DEFAULT_TMP_DIR.clone();

        Ok(Self {
            input_path: PathBuf::from("conversations.json"),
            output_dir,
            formats,
            timezone,
            date_from,
            dedupe,
            copy_assets,
            pretty_json_indent,
            filename_from,
            filters,
            state_dir,
            tmp_dir,
            source: SourceSelector::Auto,
            config_path: resolved_path,
        })
    }

    pub fn apply_cli(&mut self, cli: &Cli) -> Result<()> {
        let input = expand_path(&cli.input).context("failed to resolve input path")?;
        self.input_path = input;

        if let Some(out) = &cli.output {
            self.output_dir = expand_path(out)?;
        }

        if let Some(formats) = cli.format.as_ref() {
            self.formats = formats.iter().map(|f| OutputFormat::from_cli(*f)).collect();
        }

        if self.formats.is_empty() {
            self.formats = vec![OutputFormat::Md];
        }

        if let Some(since) = &cli.since {
            self.filters.since = parse_optional_date(Some(since))
                .with_context(|| format!("invalid --since date: {since}"))?;
        }

        if let Some(until) = &cli.until {
            self.filters.until = parse_optional_date(Some(until))
                .with_context(|| format!("invalid --until date: {until}"))?;
        }

        if let Some(tz) = &cli.timezone {
            self.timezone = Some(tz.clone());
        }

        if let Some(date_from) = cli.date_from {
            self.date_from = date_from;
        }

        if let Some(source) = cli.source {
            self.source = source;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    out_dir: Option<String>,
    #[serde(default)]
    formats: Vec<String>,
    #[serde(default)]
    tz: Option<String>,
    #[serde(default)]
    date_from: Option<String>,
    #[serde(default)]
    dedupe: Option<bool>,
    #[serde(default)]
    copy_assets: Option<bool>,
    #[serde(default)]
    pretty_json_indent: Option<usize>,
    #[serde(default)]
    filename_from: Option<String>,
    #[serde(default)]
    filters: RawFilters,
    #[serde(default)]
    state: RawState,
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            out_dir: None,
            formats: vec!["md".to_string()],
            tz: None,
            date_from: Some("utc".to_string()),
            dedupe: Some(true),
            copy_assets: Some(false),
            pretty_json_indent: Some(2),
            filename_from: Some("title".to_string()),
            filters: RawFilters::default(),
            state: RawState::default(),
        }
    }
}

impl RawConfig {
    fn merge(mut self, mut other: RawConfig) -> RawConfig {
        if other.out_dir.is_some() {
            self.out_dir = other.out_dir.take();
        }
        if !other.formats.is_empty() {
            self.formats = other.formats;
        }
        if other.tz.is_some() {
            self.tz = other.tz.take();
        }
        if other.date_from.is_some() {
            self.date_from = other.date_from.take();
        }
        if other.dedupe.is_some() {
            self.dedupe = other.dedupe.take();
        }
        if other.copy_assets.is_some() {
            self.copy_assets = other.copy_assets.take();
        }
        if other.pretty_json_indent.is_some() {
            self.pretty_json_indent = other.pretty_json_indent.take();
        }
        if other.filename_from.is_some() {
            self.filename_from = other.filename_from.take();
        }
        self.filters = self.filters.merge(other.filters);
        self.state = self.state.merge(other.state);
        self
    }

    fn resolve_formats(&self) -> Result<Vec<OutputFormat>> {
        if self.formats.is_empty() {
            return Ok(vec![OutputFormat::Md]);
        }
        let mut unique = BTreeSet::new();
        for f in &self.formats {
            let lower = f.trim().to_lowercase();
            let fmt = match lower.as_str() {
                "md" | "markdown" => OutputFormat::Md,
                "json" => OutputFormat::Json,
                other => bail!("unsupported format '{other}'"),
            };
            unique.insert(fmt);
        }
        Ok(unique.into_iter().collect())
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawFilters {
    #[serde(default)]
    since: Option<String>,
    #[serde(default)]
    until: Option<String>,
}

impl RawFilters {
    fn merge(self, other: RawFilters) -> RawFilters {
        RawFilters {
            since: other.since.or(self.since),
            until: other.until.or(self.until),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawState {
    #[serde(default)]
    dir: Option<String>,
}

impl RawState {
    fn merge(self, other: RawState) -> RawState {
        RawState {
            dir: other.dir.or(self.dir),
        }
    }
}

fn parse_optional_date(value: Option<&str>) -> Result<Option<NaiveDate>> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let date = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")?;
    Ok(Some(date))
}

fn load_raw_config(path: &Path) -> Result<RawConfig> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    let config = toml::from_str::<RawConfig>(&data)
        .with_context(|| format!("failed to parse config file {}", path.display()))?;
    Ok(config)
}

fn find_fallback_config() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(home) = home_dir() {
        let base = home.join(".floatctl");
        candidates.push(base.join("conv_split.toml"));
        candidates.push(base.join("local_config.toml"));
    }
    candidates.into_iter().find(|path| path.exists())
}
