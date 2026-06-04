//! eframe application: owns UI state, spawns the pipeline worker thread, and
//! drains pipeline events into state each frame.

use crate::pipeline::run_pipeline;
use crate::platform::{self, Platform};
use eframe::egui;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use vhdx_core::model::{Distro, PipelineEvent};

pub struct App {
    platform: Arc<dyn Platform>,
    distros: Vec<Distro>,
    selected: BTreeSet<String>,
    log: Vec<String>,
    progress: Option<(String, u32)>,
    running: bool,
    confirm_open: bool,
    rx: Option<Receiver<PipelineEvent>>,
}

/// An owned, per-frame copy of the state the UI renders from. Building this up
/// front lets `ui::draw` render without holding a borrow on `App`, so it can
/// freely apply mutations afterwards.
pub struct Snapshot {
    pub distros: Vec<Distro>,
    pub selected: BTreeSet<String>,
    pub log: Vec<String>,
    pub progress: Option<(String, u32)>,
    pub running: bool,
    pub confirm_open: bool,
}

impl App {
    pub fn new() -> Self {
        let platform: Arc<dyn Platform> = Arc::from(platform::current());
        let mut app = Self {
            platform,
            distros: Vec::new(),
            selected: BTreeSet::new(),
            log: Vec::new(),
            progress: None,
            running: false,
            confirm_open: false,
            rx: None,
        };
        app.refresh();
        app
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            distros: self.distros.clone(),
            selected: self.selected.clone(),
            log: self.log.clone(),
            progress: self.progress.clone(),
            running: self.running,
            confirm_open: self.confirm_open,
        }
    }

    pub fn toggle(&mut self, name: &str) {
        if !self.selected.remove(name) {
            self.selected.insert(name.to_string());
        }
    }

    pub fn refresh(&mut self) {
        match self.platform.list_distros() {
            Ok(d) => {
                // Drop selections that no longer exist.
                let names: BTreeSet<String> = d.iter().map(|x| x.name.clone()).collect();
                self.selected.retain(|s| names.contains(s));
                self.distros = d;
            }
            Err(e) => self.log.push(format!("discover failed: {e}")),
        }
    }

    pub fn open_confirm(&mut self) {
        self.confirm_open = true;
    }

    pub fn close_confirm(&mut self) {
        self.confirm_open = false;
    }

    pub fn confirm_and_start(&mut self, ctx: egui::Context) {
        self.confirm_open = false;
        self.start(ctx);
    }

    /// Spawn the worker thread that runs the reclaim pipeline.
    fn start(&mut self, ctx: egui::Context) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.rx = Some(rx);
        self.running = true;
        self.progress = None;
        self.log.clear();

        let platform = Arc::clone(&self.platform);
        let selected: Vec<String> = self.selected.iter().cloned().collect();
        std::thread::spawn(move || {
            run_pipeline(platform.as_ref(), &selected, &tx);
            ctx.request_repaint(); // wake the UI after the final Finished event
        });
    }

    /// Drain pipeline events into UI state (called each frame).
    fn drain_events(&mut self) {
        let Some(rx) = &self.rx else { return };
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }
        for ev in events {
            match ev {
                PipelineEvent::Discovered(d) => self.distros = d,
                PipelineEvent::TrimStarted { distro } => self.log.push(format!("trim {distro}…")),
                PipelineEvent::TrimDone { distro } => self.log.push(format!("trimmed {distro}")),
                PipelineEvent::ShutdownDone => self.log.push("WSL shut down".into()),
                PipelineEvent::CompactProgress { distro, pct } => {
                    self.progress = Some((distro, pct))
                }
                PipelineEvent::StepFailed { distro, step, err } => {
                    self.log.push(format!("⚠ {distro} {step:?} failed: {err}"))
                }
                PipelineEvent::Done {
                    distro,
                    saved,
                    engine,
                } => {
                    self.log
                        .push(format!("✓ {distro}: saved {} ({engine:?})", saved.human()));
                    self.progress = None;
                }
                PipelineEvent::Finished => {
                    self.running = false;
                    self.rx = None;
                    self.refresh();
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();
        crate::ui::draw(self, ctx);
        if self.running {
            ctx.request_repaint(); // keep polling the channel while busy
        }
    }
}
