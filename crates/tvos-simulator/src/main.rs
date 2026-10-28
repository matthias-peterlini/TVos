use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::net::UdpSocket;
use std::sync::mpsc::channel;
use tvos_apps::Desktop;
use tvos_cec::CecDriver;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_remote_web::RemoteWebServer;

const WIDTH: usize = 1280;
const HEIGHT: usize = 720;
const WEB_PORT: u16 = 8090;

fn get_local_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| s.connect("8.8.8.8:80").map(|_| s))
        .and_then(|s| s.local_addr())
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

const KEYMAP: &[(Key, RemoteKey)] = &[
    (Key::Up, RemoteKey::Up),
    (Key::Down, RemoteKey::Down),
    (Key::Left, RemoteKey::Left),
    (Key::Right, RemoteKey::Right),
    (Key::Enter, RemoteKey::Select),
    (Key::Escape, RemoteKey::Back),
    (Key::Backspace, RemoteKey::Back),
    (Key::Space, RemoteKey::PlayPause),
    (Key::H, RemoteKey::Home),
    (Key::Equal, RemoteKey::VolumeUp),
    (Key::Minus, RemoteKey::VolumeDown),
];

const KEYNAMES: &[(&str, RemoteKey)] = &[
    ("up", RemoteKey::Up),
    ("down", RemoteKey::Down),
    ("left", RemoteKey::Left),
    ("right", RemoteKey::Right),
    ("select", RemoteKey::Select),
    ("back", RemoteKey::Back),
];

fn main() {
    let local_ip = get_local_ip();
    let (tx, rx) = channel::<RemoteKey>();

    let web = RemoteWebServer::new(WEB_PORT, tx.clone());
    if let Err(e) = web.start() {
        eprintln!("[WARN] web remote: {e}");
    } else {
        println!("[WEB REMOTE] http://{}:{}  (install apps: /apps, PIN {})", local_ip, WEB_PORT, web.pin());
    }
    CecDriver::new(tx.clone()).start();

    let mut surface = Surface::new(WIDTH, HEIGHT);
    let mut desktop = Desktop::new(local_ip, WEB_PORT);
    desktop.set_pin(web.pin());

    // Headless: `--shot out.ppm [select-index]` renders one launcher frame and exits
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--shot") {
        if let Some(n) = args.get(i + 2).and_then(|v| v.parse::<usize>().ok()) {
            desktop.launcher.selected_index = n;
        }
        desktop.update(); // load installed apps before any key is pressed
        // `--keys right,down,select` presses keys first (to reach other screens)
        if let Some(k) = args.iter().position(|a| a == "--keys").and_then(|i| args.get(i + 1)) {
            for name in k.split(',') {
                if let Some((_, rk)) = KEYNAMES.iter().find(|(n, _)| *n == name) {
                    desktop.handle_key(*rk);
                }
            }
        }
        // `--wait N` waits N seconds first (lets background network fetches finish)
        if let Some(n) = args.iter().position(|a| a == "--wait").and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<u64>().ok()) {
            std::thread::sleep(std::time::Duration::from_secs(n));
        }
        desktop.settle();
        desktop.update();
        desktop.update(); // second pass gives the CPU delta a baseline
        desktop.render(&mut surface);
        let mut out = format!("P6\n{} {}\n255\n", WIDTH, HEIGHT).into_bytes();
        for px in &surface.buffer {
            out.extend_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8]);
        }
        std::fs::write(&args[i + 1], out).expect("write shot");
        return;
    }

    // Headless: `--bench` prints the average render time of the launcher and of each app
    if args.iter().any(|a| a == "--bench") {
        for (name, keys) in [("launcher", vec![]), ("youtube", vec![RemoteKey::Select])] {
            let mut d = Desktop::new("127.0.0.1".into(), WEB_PORT);
            for k in keys {
                d.handle_key(k);
            }
            d.render(&mut surface); // warm-up (glyph cache, background cache)
            let t = std::time::Instant::now();
            for _ in 0..20 {
                d.render(&mut surface);
            }
            println!("{name}: {:.1} ms/frame", t.elapsed().as_secs_f64() * 1000.0 / 20.0);
        }
        return;
    }

    let mut window = match Window::new(
        "TVos - Raspberry Pi 3B (10-Foot UI)",
        WIDTH,
        HEIGHT,
        WindowOptions { resize: false, scale: minifb::Scale::FitScreen, ..WindowOptions::default() },
    ) {
        Ok(w) => w,
        Err(err) => {
            eprintln!("[FATAL] window: {err}");
            return;
        }
    };
    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Q) {
        for (k, rk) in KEYMAP {
            if window.is_key_pressed(*k, KeyRepeat::No) {
                let _ = tx.send(*rk);
            }
        }
        while let Ok(key) = rx.try_recv() {
            desktop.handle_key(key);
        }
        if desktop.take_launch().is_some() {
            desktop.toast("Il simulatore non avvia le app: provalo sul sistema vero");
        }
        desktop.update();
        desktop.render(&mut surface);
        if window.update_with_buffer(&surface.buffer, WIDTH, HEIGHT).is_err() {
            break;
        }
    }
}
