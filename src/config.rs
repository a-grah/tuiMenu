use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

fn default_refresh() -> u64 {
    300
}
fn default_weather_location() -> String {
    "Oakville".to_string()
}
fn default_weather_format() -> String {
    "2".to_string()
}
fn default_btc_url() -> String {
    "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_weather_location")]
    pub weather_location: String,
    #[serde(default = "default_weather_format")]
    pub weather_format: String,
    #[serde(default = "default_refresh")]
    pub weather_refresh_secs: u64,
    #[serde(default = "default_refresh")]
    pub memory_refresh_secs: u64,
    #[serde(default = "default_refresh")]
    pub btc_refresh_secs: u64,
    #[serde(default = "default_btc_url")]
    pub btc_price_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            weather_location: default_weather_location(),
            weather_format: default_weather_format(),
            weather_refresh_secs: default_refresh(),
            memory_refresh_secs: default_refresh(),
            btc_refresh_secs: default_refresh(),
            btc_price_url: default_btc_url(),
        }
    }
}

/// A single config row. Either a non-selectable `heading`, or a runnable entry
/// with `name`/`description`/`command`. All fields optional so serde round-trips
/// cleanly for both shapes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Entry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

impl Entry {
    pub fn is_heading(&self) -> bool {
        self.heading.is_some()
    }

    pub fn is_runnable(&self) -> bool {
        !self.is_heading() && self.command.is_some()
    }

    pub fn name_str(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }

    pub fn description_str(&self) -> &str {
        self.description.as_deref().unwrap_or("")
    }

    pub fn command_str(&self) -> &str {
        self.command.as_deref().unwrap_or("")
    }

    pub fn heading_str(&self) -> &str {
        self.heading.as_deref().unwrap_or("")
    }

    pub fn heading(title: impl Into<String>) -> Entry {
        Entry {
            heading: Some(title.into()),
            ..Default::default()
        }
    }

    pub fn command(
        name: impl Into<String>,
        description: impl Into<String>,
        command: impl Into<String>,
    ) -> Entry {
        Entry {
            name: Some(name.into()),
            description: Some(description.into()),
            command: Some(command.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub entries: Vec<Entry>,
}

pub fn config_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".tuimenu.toml")
}

impl Config {
    /// Load from `~/.tuimenu.toml`, seeding a default file if none exists.
    pub fn load() -> Result<Config> {
        let path = config_path();
        if !path.exists() {
            let cfg = Config::seed();
            cfg.save()
                .with_context(|| format!("writing seed config to {}", path.display()))?;
            return Ok(cfg);
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading config from {}", path.display()))?;
        let cfg: Config =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        let text = toml::to_string_pretty(self).context("serializing config")?;
        fs::write(&path, text).with_context(|| format!("writing config to {}", path.display()))?;
        Ok(())
    }

    /// Starter menu, mirroring the user's rpWorkflow.conf structure but with
    /// macOS-appropriate commands so it works out of the box.
    pub fn seed() -> Config {
        Config {
            settings: Settings::default(),
            entries: vec![
                Entry::command("Help", "Show key bindings", "echo 'Edit ~/.tuimenu.toml or use o/c/dd in the TUI'"),
                Entry::heading("Manage"),
                Entry::command("On-Disk usage", "Disk usage of home dir", "du -sh $HOME"),
                Entry::command("macOS version", "Show OS version", "sw_vers"),
                Entry::heading("Financial"),
                Entry::command("BTC rate", "Live crypto rates", "curl -s rate.sx"),
                Entry::command("24 Hr Graph", "Bitcoin 24h graph", "curl -s rate.sx/BTC"),
                Entry::heading("System"),
                Entry::command("Homebrew Update", "Refresh brew formulae", "brew update"),
                Entry::command("Homebrew Upgrade", "Upgrade installed packages", "brew upgrade"),
                Entry::command("Disk free", "Free disk space", "df -h"),
                Entry::command("Top processes", "Snapshot of top processes", "top -l 1 | head -n 20"),
            ],
        }
    }
}
