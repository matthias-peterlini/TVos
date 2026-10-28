//! Installed apps: one folder per app under the apps directory, each with a `manifest.json`.
//!
//! Two kinds of app:
//! * `web`    - a name and a URL, opened in the browser (YouTube, Netflix, ... as web apps)
//! * `native` - an executable shipped inside a `.tvapp` package (a zip with `manifest.json` at its root)
//!
//! manifest.json: `{"name":"Netflix","type":"web","url":"https://www.netflix.com","color":"#E50914"}`
//! or             `{"name":"Mio gioco","type":"native","exec":"bin/gioco","color":"#3b82f6"}`

use serde_json::{json, Value};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

/// Largest package accepted and largest amount of data it may unpack to.
pub const MAX_PACKAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_UNPACKED_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppKind {
    Web,
    Native,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub kind: AppKind,
    pub url: String,
    /// Absolute path of the executable (native apps)
    pub exec: PathBuf,
    /// Accent colour 0xRRGGBB
    pub color: u32,
}

/// Where apps live: `$TVOS_APPS_DIR`, `/var/lib/tvos/apps` on the device, `.tvos-apps` elsewhere.
pub fn apps_dir() -> PathBuf {
    if let Ok(d) = std::env::var("TVOS_APPS_DIR") {
        return PathBuf::from(d);
    }
    if cfg!(target_os = "linux") && Path::new("/var/lib").is_dir() {
        PathBuf::from("/var/lib/tvos/apps")
    } else {
        PathBuf::from(".tvos-apps")
    }
}

pub fn parse_color(s: &str) -> Option<u32> {
    let h = s.trim().trim_start_matches('#');
    (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok()).flatten()
}

/// "Disney+ Hotstar!" -> "disney-hotstar"
pub fn make_id(name: &str) -> String {
    let mut id = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            id.push(c.to_ascii_lowercase());
        } else if !id.ends_with('-') && !id.is_empty() {
            id.push('-');
        }
    }
    id.trim_end_matches('-').chars().take(40).collect()
}

fn read_manifest(dir: &Path, id: &str) -> Result<InstalledApp, String> {
    let text = fs::read_to_string(dir.join(id).join("manifest.json")).map_err(|e| format!("manifest.json: {e}"))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("manifest.json non valido: {e}"))?;
    let name = v["name"].as_str().map(str::trim).filter(|n| !n.is_empty() && n.chars().count() <= 40);
    let name = name.ok_or("manifest: nome mancante o troppo lungo")?.to_string();
    let color = v["color"].as_str().and_then(parse_color).unwrap_or(0x6366F1);
    match v["type"].as_str() {
        Some("web") => {
            let url = v["url"].as_str().unwrap_or("").trim().to_string();
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                return Err("manifest: l'indirizzo deve iniziare con https://".into());
            }
            Ok(InstalledApp { id: id.into(), name, kind: AppKind::Web, url, exec: PathBuf::new(), color })
        }
        Some("native") => {
            let exec = v["exec"].as_str().ok_or("manifest: manca \"exec\"")?;
            let rel = Path::new(exec);
            let safe = rel.components().all(|c| matches!(c, std::path::Component::Normal(_)));
            if !safe || rel.as_os_str().is_empty() {
                return Err("manifest: percorso exec non valido".into());
            }
            Ok(InstalledApp { id: id.into(), name, kind: AppKind::Native, url: String::new(), exec: dir.join(id).join(rel), color })
        }
        _ => Err("manifest: \"type\" deve essere \"web\" o \"native\"".into()),
    }
}

/// All valid installed apps, sorted by name. Broken folders are skipped.
pub fn list(dir: &Path) -> Vec<InstalledApp> {
    let mut apps: Vec<InstalledApp> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().into_owned();
            if id.starts_with('.') || !e.path().is_dir() {
                return None;
            }
            read_manifest(dir, &id).ok()
        })
        .collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

pub fn install_web(dir: &Path, name: &str, url: &str, color: Option<u32>) -> Result<InstalledApp, String> {
    let id = make_id(name);
    if id.is_empty() {
        return Err("Nome non valido".into());
    }
    let manifest = json!({
        "name": name.trim(),
        "type": "web",
        "url": url.trim(),
        "color": format!("#{:06X}", color.unwrap_or(0x6366F1)),
    });
    let target = dir.join(&id);
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    fs::write(target.join("manifest.json"), manifest.to_string()).map_err(|e| e.to_string())?;
    read_manifest(dir, &id).inspect_err(|_| {
        let _ = fs::remove_dir_all(&target);
    })
}

/// Install a `.tvapp` package (zip). Nothing is left behind if anything is wrong with it.
pub fn install_package(dir: &Path, bytes: &[u8]) -> Result<InstalledApp, String> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err("Pacchetto troppo grande".into());
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| "Non è un pacchetto .tvapp valido (zip)".to_string())?;

    // The manifest decides the app id and kind
    let mut manifest_text = String::new();
    zip.by_name("manifest.json")
        .map_err(|_| "Nel pacchetto manca manifest.json")?
        .take(64 * 1024)
        .read_to_string(&mut manifest_text)
        .map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&manifest_text).map_err(|e| format!("manifest.json non valido: {e}"))?;
    let id = make_id(v["name"].as_str().unwrap_or(""));
    if id.is_empty() {
        return Err("manifest: nome mancante".into());
    }

    let total: u64 = (0..zip.len()).filter_map(|i| zip.by_index(i).ok().map(|f| f.size())).sum();
    if total > MAX_UNPACKED_BYTES {
        return Err("Pacchetto troppo grande una volta decompresso".into());
    }

    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let staging = dir.join(format!(".{id}.installing"));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;

    let result = (|| -> Result<(), String> {
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
            // enclosed_name rejects absolute paths and "..": no file can land outside the app folder
            let rel = f.enclosed_name().ok_or("Il pacchetto contiene un percorso non sicuro")?;
            let out = staging.join(&rel);
            if f.is_dir() {
                fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut file = fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut f.by_ref().take(MAX_UNPACKED_BYTES), &mut file).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    let finish = result.and_then(|_| {
        // validate with the same rules used at list time, from inside the staging folder
        read_manifest(dir, &format!(".{id}.installing")).map(|_| ())?;
        let v: Value = serde_json::from_str(&manifest_text).map_err(|e| e.to_string())?;
        if v["type"] == "native" {
            let exec = staging.join(v["exec"].as_str().unwrap_or(""));
            if !exec.is_file() {
                return Err("Il file eseguibile indicato in \"exec\" non è nel pacchetto".into());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&exec, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    });
    if let Err(e) = finish {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    let target = dir.join(&id);
    let _ = fs::remove_dir_all(&target); // reinstalling replaces the old version
    fs::rename(&staging, &target).map_err(|e| e.to_string())?;
    read_manifest(dir, &id)
}

pub fn remove(dir: &Path, id: &str) -> Result<(), String> {
    if id.is_empty() || id != make_id(id) {
        return Err("Identificativo non valido".into());
    }
    fs::remove_dir_all(dir.join(id)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tvos-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in files {
            w.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn ids_and_colors() {
        assert_eq!(make_id("Disney+ Hotstar!"), "disney-hotstar");
        assert_eq!(make_id("  "), "");
        assert_eq!(parse_color("#E50914"), Some(0xE50914));
        assert_eq!(parse_color("zzz"), None);
    }

    #[test]
    fn web_app_roundtrip() {
        let d = tmp("web");
        let app = install_web(&d, "Netflix", "https://www.netflix.com", Some(0xE50914)).unwrap();
        assert_eq!((app.id.as_str(), app.kind, app.color), ("netflix", AppKind::Web, 0xE50914));
        assert_eq!(list(&d).len(), 1);
        remove(&d, "netflix").unwrap();
        assert!(list(&d).is_empty());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn rejects_bad_web_input() {
        let d = tmp("badweb");
        assert!(install_web(&d, "X", "javascript:alert(1)", None).is_err());
        assert!(install_web(&d, "!!!", "https://a.b", None).is_err());
        assert!(list(&d).is_empty(), "failed installs must leave nothing behind");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn native_package_installs_and_is_executable() {
        let d = tmp("native");
        let pkg = zip_of(&[
            ("manifest.json", br##"{"name":"Gioco","type":"native","exec":"bin/gioco","color":"#3b82f6"}"##),
            ("bin/gioco", b"#!/bin/sh\necho ciao\n"),
        ]);
        let app = install_package(&d, &pkg).unwrap();
        assert_eq!(app.kind, AppKind::Native);
        assert!(app.exec.is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&app.exec).unwrap().permissions().mode() & 0o111, 0o111);
        }
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn package_errors_leave_nothing_behind() {
        let d = tmp("bad");
        assert!(install_package(&d, b"not a zip").is_err());
        let no_manifest = zip_of(&[("a.txt", b"x")]);
        assert!(install_package(&d, &no_manifest).is_err());
        let missing_exec = zip_of(&[("manifest.json", br#"{"name":"A","type":"native","exec":"bin/a"}"#)]);
        assert!(install_package(&d, &missing_exec).is_err());
        let evil_exec = zip_of(&[("manifest.json", br#"{"name":"A","type":"native","exec":"../../bin/sh"}"#)]);
        assert!(install_package(&d, &evil_exec).is_err());
        assert!(list(&d).is_empty());
        assert!(fs::read_dir(&d).map_or(true, |mut r| r.next().is_none()), "no staging folders left");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn zip_slip_is_blocked() {
        let d = tmp("slip");
        let pkg = zip_of(&[
            ("manifest.json", br#"{"name":"Evil","type":"web","url":"https://a.b"}"#),
            ("../escaped.txt", b"pwned"),
        ]);
        assert!(install_package(&d, &pkg).is_err());
        assert!(!d.parent().unwrap().join("escaped.txt").exists());
        let _ = fs::remove_dir_all(&d);
    }
}
