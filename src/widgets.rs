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

fn mem_gib(sys: &System) -> (f64, f64) {
    (sys.used_memory() as f64 / GIB, sys.total_memory() as f64 / GIB)
}

pub fn memory_string(sys: &System) -> String {
    let (used, total) = mem_gib(sys);
    format!("{used:.1}/{total:.1} GB")
}

pub fn uptime_string() -> String {
    let secs = System::uptime();
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        format!("up {days}d {hours}h")
    } else if hours > 0 {
        format!("up {hours}h {mins}m")
    } else {
        format!("up {mins}m")
    }
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
    mem_used: f64,  // GiB
    mem_total: f64, // GiB
    last_mem: Instant,
    mem_interval: Duration,
    weather: Arc<Mutex<Option<String>>>,
    btc: Arc<Mutex<Option<String>>>,
}

impl Widgets {
    pub fn new(settings: &Settings) -> Widgets {
        let mut sys = System::new();
        sys.refresh_memory();
        let (mem_used, mem_total) = mem_gib(&sys);

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
            mem_used,
            mem_total,
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
            (self.mem_used, self.mem_total) = mem_gib(&self.sys);
            self.last_mem = Instant::now();
        }
    }

    pub fn memory_str(&self) -> String {
        format!("{:.1}/{:.1} GB", self.mem_used, self.mem_total)
    }

    pub fn memory_ratio(&self) -> f64 {
        if self.mem_total == 0.0 {
            0.0
        } else {
            (self.mem_used / self.mem_total).clamp(0.0, 1.0)
        }
    }

    pub fn uptime_str(&self) -> String {
        uptime_string()
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
