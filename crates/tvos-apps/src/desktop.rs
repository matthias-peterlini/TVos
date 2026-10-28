//! The whole "desktop": home screen + apps, shared by the simulator and the real shell.

use crate::apps_screen::AppsScreen;
use crate::nav::{self, ease, Tab};
use crate::setup::SetupScreen;
use crate::sources::SourcesScreen;
use crate::{ActiveScreen, Launch, Launcher, SettingsApp, SysInfo, SystemMonitor, YouTubeApp};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tvos_core::color::Color;
use tvos_core::input::RemoteKey;
use tvos_core::surface::Surface;
use tvos_core::text::{text_width, Weight};

pub struct Desktop {
    pub launcher: Launcher,
    youtube: YouTubeApp,
    system_monitor: SystemMonitor,
    settings: SettingsApp,
    sources: SourcesScreen,
    /// First-boot screen with the phone-remote QR code; None once completed
    setup: Option<SetupScreen>,
    apps_screen: AppsScreen,
    sys: SysInfo,
    apps_dir: PathBuf,
    last_refresh: Instant,
    toast: Option<(String, Instant)>,
    tab: Tab,
    /// Animated position of the pill in the top bar (0 = home)
    tab_pos: f32,
    bar_focus: bool,
    last_tick: Instant,
    animating: bool,
}

impl Desktop {
    pub fn new(local_ip: String, web_port: u16) -> Self {
        let mut launcher = Launcher::new(local_ip.clone());
        launcher.web_port = web_port;
        let apps_screen = AppsScreen { ip: local_ip.clone(), port: web_port, ..Default::default() };
        let mut settings = SettingsApp::new(local_ip, web_port);
        settings.name = launcher.name.clone();
        let apps_dir = tvos_store::apps_dir();
        let setup = (!setup_marker(&apps_dir).exists() || std::env::var_os("TVOS_SETUP").is_some())
            .then(|| SetupScreen { port: web_port, ..Default::default() });
        Self {
            launcher,
            youtube: YouTubeApp::new(),
            system_monitor: SystemMonitor::new(),
            settings,
            sources: SourcesScreen::default(),
            setup,
            apps_screen,
            sys: SysInfo::new(),
            apps_dir,
            last_refresh: Instant::now() - Duration::from_secs(10),
            toast: None,
            tab: Tab::Home,
            tab_pos: 0.0,
            bar_focus: false,
            last_tick: Instant::now(),
            animating: false,
        }
    }

    /// PIN the phone must type to install apps (shown on the "Installa app" screen)
    pub fn set_pin(&mut self, pin: &str) {
        self.apps_screen.pin = pin.to_string();
        self.settings.pin = pin.to_string();
        if let Some(setup) = &mut self.setup {
            setup.pin = pin.to_string();
        }
    }

    /// A short message at the top of the screen for a few seconds
    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    /// The app the user just opened, if it must be started outside TVos' own screens.
    pub fn take_launch(&mut self) -> Option<Launch> {
        self.launcher.pending_launch.take().or_else(|| self.youtube.take_launch())
    }

    /// True while an animation runs: the shell then redraws at ~60 fps instead of waiting for the idle tick.
    pub fn animating(&self) -> bool {
        self.animating
    }

    /// Jump to the end of every animation (screenshots)
    pub fn settle(&mut self) {
        self.tab_pos = self.tab.index() as f32;
        self.launcher.settle();
        self.animating = false;
    }

    pub fn handle_key(&mut self, key: RemoteKey) {
        if let Some(setup) = &mut self.setup {
            if setup.handle_key(key) {
                self.setup = None;
                let marker = setup_marker(&self.apps_dir);
                if let Some(dir) = marker.parent().filter(|d| !d.as_os_str().is_empty()) {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(marker, b"ok\n");
            }
            return;
        }
        let l = &mut self.launcher;
        match l.active_screen {
            ActiveScreen::Launcher => self.handle_nav_key(key),
            ActiveScreen::YouTube => {
                if key == RemoteKey::Home || (!self.youtube.detail_open && key == RemoteKey::Back) {
                    l.active_screen = ActiveScreen::Launcher;
                } else {
                    self.youtube.handle_key(key);
                }
            }
            ActiveScreen::SystemMonitor | ActiveScreen::AddApps => {
                if matches!(key, RemoteKey::Back | RemoteKey::Home) {
                    l.active_screen = ActiveScreen::Launcher;
                } else {
                    match l.active_screen {
                        ActiveScreen::SystemMonitor => self.system_monitor.handle_key(key),
                        _ => self.apps_screen.handle_key(key),
                    };
                }
            }
            // Retired screens: never reachable from the home screen any more
            ActiveScreen::Settings | ActiveScreen::PrimeVideo | ActiveScreen::ScreenMirror | ActiveScreen::MediaPlayer => {
                l.active_screen = ActiveScreen::Launcher
            }
        }
    }

    fn handle_nav_key(&mut self, key: RemoteKey) {
        if key == RemoteKey::Home {
            self.tab = Tab::Home;
            self.bar_focus = false;
            self.launcher.reset();
            return;
        }
        if self.bar_focus {
            let i = self.tab.index();
            match key {
                RemoteKey::Left if i > 0 => self.tab = Tab::ALL[i - 1],
                RemoteKey::Right if i + 1 < Tab::ALL.len() => self.tab = Tab::ALL[i + 1],
                RemoteKey::Down | RemoteKey::Select => self.bar_focus = false,
                _ => {}
            }
            return;
        }
        if key == RemoteKey::Back && self.tab != Tab::Home {
            self.tab = Tab::Home;
            return;
        }
        let up = match self.tab {
            Tab::Home => self.launcher.handle_key(key),
            Tab::Sources => self.sources.handle_key(key),
            Tab::Settings => self.settings.handle_key(key),
        };
        if up {
            self.bar_focus = true;
        }
    }

    pub fn update(&mut self) {
        let dt = self.last_tick.elapsed().as_secs_f32().min(0.1);
        self.last_tick = Instant::now();
        let mut moving = ease(&mut self.tab_pos, self.tab.index() as f32, dt, 12.0);
        moving |= self.launcher.tick(dt);
        self.animating = moving;

        if self.last_refresh.elapsed() >= Duration::from_secs(2) {
            self.sys.refresh();
            self.launcher.sys = self.sys.clone();
            self.system_monitor.sys = self.sys.clone();
            self.settings.sys = self.sys.clone();
            self.sources.sys = self.sys.clone();
            if let Some(setup) = &mut self.setup {
                setup.sys = self.sys.clone();
            }
            let installed = tvos_store::list(&self.apps_dir);
            self.apps_screen.installed = installed.iter().map(|a| a.name.clone()).collect();
            self.settings.installed_apps = installed.len();
            if let Some(ip) = &self.sys.ip {
                self.apps_screen.ip = ip.clone();
                self.settings.server_ip = ip.clone();
            }
            self.launcher.set_installed(installed);
            self.last_refresh = Instant::now();
        }
        if matches!(&self.toast, Some((_, t)) if t.elapsed() > Duration::from_secs(5)) {
            self.toast = None;
        }
    }

    pub fn render(&mut self, s: &mut Surface) {
        if let Some(setup) = &self.setup {
            setup.render(s);
            return;
        }
        match self.launcher.active_screen {
            ActiveScreen::Launcher => {
                // The content switches halfway through the pill's glide
                let shown = Tab::ALL[(self.tab_pos.round() as usize).min(Tab::ALL.len() - 1)];
                match shown {
                    Tab::Home => self.launcher.render(s),
                    Tab::Sources => {
                        s.draw_background();
                        self.sources.active = !self.bar_focus;
                        self.sources.render(s);
                    }
                    Tab::Settings => {
                        s.draw_background();
                        self.settings.render(s);
                    }
                }
                if !(shown == Tab::Home && self.launcher.in_library()) {
                    nav::draw_bar(s, self.tab_pos, self.bar_focus);
                    nav::draw_chevron(s);
                }
            }
            ActiveScreen::YouTube => self.youtube.render(s),
            ActiveScreen::SystemMonitor => self.system_monitor.render(s),
            ActiveScreen::AddApps => self.apps_screen.render(s),
            ActiveScreen::Settings | ActiveScreen::PrimeVideo | ActiveScreen::ScreenMirror | ActiveScreen::MediaPlayer => self.launcher.render(s),
        }
        if let Some((msg, _)) = &self.toast {
            let size = 18.0 * s.height as f32 / 720.0;
            let tw = text_width(msg, size, Weight::SemiBold);
            let (w, h) = (tw + 56, (size * 2.3) as i32);
            let x = (s.width as i32 - w) / 2;
            let y = s.height as i32 / 22; // top centre: the bottom is taken by the dock and the app name
            s.fill_rrect(x, y, w as u32, h as u32, h / 2, Color::rgba(15, 15, 20, 230));
            s.stroke_rrect(x, y, w as u32, h as u32, h / 2, 1.0, Color::rgba(255, 255, 255, 60));
            s.draw_text_ex(x + 28, y + (h as f32 * 0.28) as i32, msg, Color::white(), size, Weight::SemiBold);
        }
    }
}

/// Written when the first-boot setup is completed: next to the apps folder, so it survives reboots.
fn setup_marker(apps_dir: &std::path::Path) -> PathBuf {
    apps_dir.with_file_name(".tvos-setup-done")
}
