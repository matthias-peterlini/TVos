//! Real system data read from /proc and /sys. Every field is optional: on a machine that does
//! not expose it (the Windows simulator, a VM without sensors) the UI shows "n/d" instead of a made-up value.

use std::fs;
use std::net::UdpSocket;

#[derive(Clone, Debug, Default)]
pub struct NetIface {
    pub name: String,
    pub wireless: bool,
    pub up: bool,
}

/// A video output connector from /sys/class/drm (HDMI on the Pi)
#[derive(Clone, Debug, Default)]
pub struct Connector {
    pub name: String,
    pub connected: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SysInfo {
    pub hardware: Option<String>,
    pub kernel: Option<String>,
    pub cpu_cores: usize,
    pub cpu_percent: Option<f32>,
    pub ram_total_mb: Option<u32>,
    pub ram_used_mb: Option<u32>,
    pub temp_c: Option<f32>,
    pub uptime_secs: Option<u64>,
    pub ip: Option<String>,
    pub ifaces: Vec<NetIface>,
    pub bluetooth: bool,
    pub connectors: Vec<Connector>,
    /// A DVB (digital terrestrial) tuner is present
    pub dvb: bool,
    /// HDMI-CEC adapter registered and talking to the TV (written by the CEC driver thread)
    pub cec_ready: bool,
    pub cec_present: bool,
    /// The address is kept across reboots (written by /etc/init.d/S41network)
    pub static_ip: bool,
    cpu_prev: Option<(u64, u64)>, // (busy, total) jiffies
}

fn read(path: &str) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn cpu_jiffies() -> Option<(u64, u64)> {
    let line = fs::read_to_string("/proc/stat").ok()?;
    let v: Vec<u64> = line.lines().next()?.split_whitespace().skip(1).filter_map(|n| n.parse().ok()).collect();
    if v.len() < 5 {
        return None;
    }
    let total: u64 = v.iter().take(8).sum();
    let idle = v[3] + v[4]; // idle + iowait
    Some((total - idle, total))
}

fn meminfo(key: &str, text: &str) -> Option<u32> {
    let l = text.lines().find(|l| l.starts_with(key))?;
    Some((l.split_whitespace().nth(1)?.parse::<u64>().ok()? / 1024) as u32)
}

pub fn local_ip() -> Option<String> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

impl SysInfo {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.hardware = read("/proc/device-tree/model")
            .map(|m| m.trim_end_matches('\0').to_string())
            .or_else(|| {
                let c = fs::read_to_string("/proc/cpuinfo").ok()?;
                let l = c.lines().find(|l| l.starts_with("model name"))?;
                Some(l.split(':').nth(1)?.trim().to_string())
            });
        s.kernel = read("/proc/sys/kernel/osrelease");
        s.cpu_cores = fs::read_to_string("/proc/cpuinfo")
            .map(|c| c.lines().filter(|l| l.starts_with("processor")).count())
            .unwrap_or(0);
        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        if let Some(now) = cpu_jiffies() {
            if let Some(prev) = self.cpu_prev {
                let (db, dt) = (now.0.saturating_sub(prev.0), now.1.saturating_sub(prev.1));
                if dt > 0 {
                    self.cpu_percent = Some(db as f32 * 100.0 / dt as f32);
                }
            }
            self.cpu_prev = Some(now);
        }
        if let Ok(m) = fs::read_to_string("/proc/meminfo") {
            let total = meminfo("MemTotal:", &m);
            let avail = meminfo("MemAvailable:", &m);
            self.ram_total_mb = total;
            self.ram_used_mb = total.zip(avail).map(|(t, a)| t.saturating_sub(a));
        }
        self.temp_c = read("/sys/class/thermal/thermal_zone0/temp")
            .and_then(|t| t.parse::<f32>().ok())
            .map(|t| t / 1000.0);
        self.uptime_secs = read("/proc/uptime").and_then(|u| u.split('.').next()?.parse().ok());
        self.ip = local_ip();

        self.ifaces.clear();
        if let Ok(dir) = fs::read_dir("/sys/class/net") {
            for e in dir.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name == "lo" {
                    continue;
                }
                let base = format!("/sys/class/net/{name}");
                self.ifaces.push(NetIface {
                    wireless: fs::metadata(format!("{base}/wireless")).is_ok(),
                    up: read(&format!("{base}/operstate")).map_or(false, |s| s == "up" || s == "unknown"),
                    name,
                });
            }
        }
        self.bluetooth = fs::metadata("/sys/class/bluetooth").map_or(false, |m| {
            m.is_dir() && fs::read_dir("/sys/class/bluetooth").map_or(false, |mut d| d.next().is_some())
        });
        self.connectors.clear();
        if let Ok(dir) = fs::read_dir("/sys/class/drm") {
            for e in dir.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if let Some((_, conn)) = n.split_once('-') {
                    self.connectors.push(Connector { name: conn.to_string(), connected: read(&format!("/sys/class/drm/{n}/status")).map_or(false, |s| s == "connected") });
                }
            }
        }
        self.dvb = fs::read_dir("/dev/dvb").map_or(false, |mut d| d.next().is_some());
        self.cec_present = fs::metadata("/dev/cec0").is_ok();
        self.cec_ready = read("/run/tvos-cec").map_or(false, |s| s == "ok");
        self.static_ip = read("/run/tvos-net").map_or(false, |s| s.starts_with("static"));
    }

    pub fn has_wifi(&self) -> bool {
        self.ifaces.iter().any(|i| i.wireless)
    }

    /// Short description of the active connection for the home screen
    pub fn network_summary(&self) -> (String, String) {
        match (&self.ip, self.ifaces.iter().find(|i| i.up)) {
            (Some(ip), Some(i)) if i.wireless => ("Wi-Fi connesso".into(), format!("{} - {ip}", i.name)),
            (Some(ip), Some(i)) => ("Rete cablata connessa".into(), format!("{} - {ip}", i.name)),
            (Some(ip), None) => ("Rete connessa".into(), ip.clone()),
            _ if self.has_wifi() => ("Wi-Fi non connesso".into(), "Nessuna rete configurata".into()),
            _ => ("Nessuna rete".into(), "Cavo scollegato o nessun adattatore".into()),
        }
    }

    pub fn uptime_text(&self) -> String {
        match self.uptime_secs {
            Some(s) if s >= 3600 => format!("{} h {} min", s / 3600, (s % 3600) / 60),
            Some(s) => format!("{} min {} s", s / 60, s % 60),
            None => "n/d".into(),
        }
    }
}
