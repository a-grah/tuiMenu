use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use sysinfo::System;

use crate::config::Settings;

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

fn curl(url: &str) -> Option<String> {
    let out = Command::new("curl")
        .arg("-s")
        .arg("--max-time")
        .arg("10")
        .arg(url)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn with_commas(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    let bytes = s.as_bytes();
    for (i, c) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*c as char);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

fn parse_btc(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let price = v.get("bitcoin")?.get("usd")?.as_f64()?;
    Some(format!("BTC ${}", with_commas(price.round() as i64)))
}

pub fn fetch_weather(settings: &Settings) -> Option<String> {
    let url = format!(
        "https://wttr.in/{}?format={}",
        settings.weather_location, settings.weather_format
    );
    curl(&url)
}

pub fn fetch_btc(settings: &Settings) -> Option<String> {
    let json = curl(&settings.btc_price_url)?;
    parse_btc(&json)
}

pub fn memory_string(sys: &System) -> String {
    let used = sys.used_memory() as f64 / GIB;
    let total = sys.total_memory() as f64 / GIB;
    format!("Mem {used:.1}/{total:.1} GB")
}

pub fn short_hostname() -> String {
    System::host_name()
        .map(|h| h.split('.').next().unwrap_or(&h).to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "localhost".to_string())
}

pub struct Widgets {
    pub hostname: String,
    sys: System,
    memory: String,
    last_mem: Instant,
    mem_interval: Duration,
    weather: Arc<Mutex<Option<String>>>,
    btc: Arc<Mutex<Option<String>>>,
}

impl Widgets {
    pub fn new(settings: &Settings) -> Widgets {
        let mut sys = System::new();
        sys.refresh_memory();
        let memory = memory_string(&sys);

        let weather = Arc::new(Mutex::new(None));
        let btc = Arc::new(Mutex::new(None));

        spawn_poller(weather.clone(), settings.weather_refresh_secs, {
            let s = settings.clone();
            move || fetch_weather(&s)
        });
        spawn_poller(btc.clone(), settings.btc_refresh_secs, {
            let s = settings.clone();
            move || fetch_btc(&s)
        });

        Widgets {
            hostname: short_hostname(),
            sys,
            memory,
            last_mem: Instant::now(),
            mem_interval: Duration::from_secs(settings.memory_refresh_secs.max(1)),
            weather,
            btc,
        }
    }

    /// Recompute the memory reading if its refresh interval has elapsed.
    pub fn tick(&mut self) {
        if self.last_mem.elapsed() >= self.mem_interval {
            self.sys.refresh_memory();
            self.memory = memory_string(&self.sys);
            self.last_mem = Instant::now();
        }
    }

    pub fn memory_str(&self) -> &str {
        &self.memory
    }

    pub fn weather_str(&self) -> String {
        self.weather
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_else(|| "weather …".to_string())
    }

    pub fn btc_str(&self) -> String {
        self.btc
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_else(|| "BTC …".to_string())
    }
}

fn spawn_poller<F>(slot: Arc<Mutex<Option<String>>>, interval_secs: u64, fetch: F)
where
    F: Fn() -> Option<String> + Send + 'static,
{
    let interval = Duration::from_secs(interval_secs.max(1));
    thread::spawn(move || loop {
        if let Some(v) = fetch() {
            if let Ok(mut g) = slot.lock() {
                *g = Some(v);
            }
        }
        thread::sleep(interval);
    });
}
