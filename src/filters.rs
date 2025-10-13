use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, Utc};
use chrono_tz::Tz;

use crate::cli::DateFrom;
use crate::config::{Config, FiltersConfig};
use crate::model::Conversation;

pub struct FilterContext<'a> {
    config: &'a Config,
    tz: Option<Tz>,
}

impl<'a> FilterContext<'a> {
    pub fn new(config: &'a Config) -> Result<Self> {
        let tz = match config.timezone.as_deref() {
            Some(name) if !name.is_empty() => Some(
                name.parse::<Tz>()
                    .with_context(|| format!("unknown timezone '{}'", name))?,
            ),
            _ => None,
        };
        Ok(Self { config, tz })
    }

    pub fn includes(&self, conversation: &Conversation) -> bool {
        within_date_filters(conversation, &self.config.filters)
    }

    pub fn filename_prefix_date(&self, created: DateTime<Utc>) -> NaiveDate {
        match self.config.date_from {
            DateFrom::Utc => created.date_naive(),
            DateFrom::Local => {
                if let Some(tz) = self.tz {
                    created.with_timezone(&tz).date_naive()
                } else {
                    created.with_timezone(&Local::now().timezone()).date_naive()
                }
            }
        }
    }

    pub fn display_timestamp(&self, timestamp: Option<DateTime<Utc>>) -> Option<String> {
        timestamp.map(|ts| {
            if let Some(tz) = self.tz {
                ts.with_timezone(&tz)
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            } else {
                ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            }
        })
    }
}

fn within_date_filters(conversation: &Conversation, filters: &FiltersConfig) -> bool {
    if let Some(since) = filters.since {
        if conversation.created.date_naive() < since {
            return false;
        }
    }
    if let Some(until) = filters.until {
        if conversation.created.date_naive() > until {
            return false;
        }
    }
    true
}
