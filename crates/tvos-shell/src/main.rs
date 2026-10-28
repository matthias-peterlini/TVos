//! TVos shell: the single full-screen program that runs after boot.

mod fb;
mod input;
mod launch;
mod uinput;

use std::net::UdpSocket;
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tvos_apps::Desktop;
use tvos_cec::CecDriver;
use tvos_core::color::Color;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_remote_web::RemoteWebServer;

const WEB_PORT: u16 = 8080;
const IDLE_TICK: Duration = Duration::from_millis(250);

fn local_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| s.connect("8.8.8.8:80").map(|_| s))
        .and_then(|s| s.local_addr())
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| "0.0.0.0".to_string())
}

fn main() {
    let mut screen = loop {
        match fb::Fb::open() {
            Ok(f) => break f,
            Err(e) => {
                eprintln!("[tvos] waiting for /dev/fb0: {e}");
                sleep(Duration::from_secs(1));
            }
        }
    };
    let (tx, rx) = channel::<RemoteKey>();
    let n = input::spawn(tx.clone());
    eprintln!("[tvos] {}x{} fb, {n} input devices", screen.width, screen.height);
    let web = RemoteWebServer::new(WEB_PORT, tx.clone());
    if let Err(e) = web.start() {
        eprintln!("[tvos] web remote: {e}");
    }
    CecDriver::new(tx.clone()).start();

    let mut surface = Surface::new(screen.width, screen.height);
    let mut desktop = Desktop::new(local_ip(), WEB_PORT);
    desktop.set_pin(web.pin());
    let mut last_ip_check = Instant::now();
    let mut runner = launch::Runner::default();
    // `tvos.debug` on the kernel command line shows frame timings on screen
    let debug = std::fs::read_to_string("/proc/cmdline").map_or(false, |c| c.contains("tvos.debug"));
    let (mut present_ms, mut frame_ms) = (0.0f32, 0.0f32);

    loop {
        // Sleep until a key arrives (instant reaction) or the idle tick fires (clock, animations).
        let first = rx.recv_timeout(if desktop.animating() { Duration::from_millis(16) } else { IDLE_TICK });
        let t0 = Instant::now();
        if let Ok(k) = first {
            desktop.handle_key(k);
        }
        while let Ok(k) = rx.try_recv() {
            desktop.handle_key(k);
        }
        if let Some(l) = desktop.take_launch() {
            // The app owns the screen while it runs; afterwards repaint everything and drop keys typed meanwhile
            if let Err(msg) = runner.run(&l, &rx) {
                desktop.toast(msg);
            }
            screen.invalidate();
            while rx.try_recv().is_ok() {}
        }
        if last_ip_check.elapsed() > Duration::from_secs(5) {
            desktop.launcher.server_ip = local_ip();
            last_ip_check = Instant::now();
        }
        desktop.update();
        let t_render = Instant::now();
        desktop.render(&mut surface);
        let render_ms = t_render.elapsed().as_secs_f32() * 1000.0;
        if debug {
            let msg = format!("render {render_ms:.0} ms | present {present_ms:.0} ms | frame {frame_ms:.0} ms");
            surface.draw_text(8, 6, &msg, Color::rgb(255, 255, 0), 1);
        }
        let t_present = Instant::now();
        screen.present(&surface.buffer);
        present_ms = t_present.elapsed().as_secs_f32() * 1000.0;
        frame_ms = t0.elapsed().as_secs_f32() * 1000.0;
    }
}
