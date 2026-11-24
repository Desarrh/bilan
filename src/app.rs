//! État de la fenêtre, chargement en arrière-plan et dessin de la page de bilan.

use crate::constats::{constats, un};
use crate::modele::{Semaine, lundi_de};
use crate::sources::{git, jeu, memoire, sport};
use chrono::{Duration, Local, NaiveDate};
use egui::{Align, Color32, CornerRadius, FontId, Frame, Layout, Margin, RichText, Sense, Ui, vec2};
use serde::{Deserialize, Serialize};
use socle::icons::{self, Icon, Look};
use socle::theme::{self, Palette, taille};
use socle::ui::{Command, Toasts};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

pub const APP: &str = "Bilan";
const SEMAINES: usize = 12;

pub static UPDATER: socle::update::Updater = socle::update::Updater {
    repo: "Desarrh/bilan",
    current: env!("CARGO_PKG_VERSION"),
    asset_suffix: "windows-x64.zip",
    exe_name: None,
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub dark: Option<bool>,
    /// Tâche planifiée d'ouverture du lundi (désactivée par défaut).
    pub auto: bool,
    pub auteur: String,
    pub racines: Vec<String>,
    pub exclure: Vec<String>,
    pub memoire_exe: Option<String>,
    /// Lundi (AAAA-MM-JJ) du dernier bilan complet affiché.
    pub dernier_vu: Option<String>,
    pub window: Option<(f32, f32)>,
    pub last_update_check: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            dark: None,
            auto: false,
            auteur: "Desarrh".into(),
            racines: vec![r"C:\Dev".into()],
            exclure: vec!["godot".into()],
            memoire_exe: None,
            dernier_vu: None,
            window: None,
            last_update_check: 0,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        socle::settings::load(APP)
    }
}

/// Lundi de la dernière semaine terminée.
pub fn derniere_semaine() -> NaiveDate {
    lundi_de(Local::now().date_naive()) - Duration::days(7)
}

pub fn photos_jeu() -> PathBuf {
    socle::paths::config_dir(APP).join("jeu-photos.json")
}

/// Rassemble toutes les sources (appelé hors du fil de l'interface).
pub fn charger(s: &Settings) -> Vec<Semaine> {
    let courant = lundi_de(Local::now().date_naive());
    let lundis: Vec<NaiveDate> = (0..SEMAINES).rev().map(|k| courant - Duration::days(7 * k as i64)).collect();
    let ult = jeu::dossier_ultimate();
    jeu::photographier(&ult, &photos_jeu(), Local::now().date_naive());
    let mexe = memoire::trouver(s.memoire_exe.as_deref());
    let fsport = sport::trouver(&sport::dossiers());
    let r = git::Reglages { racines: &s.racines, auteur: &s.auteur, exclure: &s.exclure };
    let (d, j, sp, m) = std::thread::scope(|t| {
        let d = t.spawn(|| git::charger(&lundis, &r));
        let j = t.spawn(|| jeu::charger(&lundis, &ult, &photos_jeu()));
        let sp = t.spawn(|| sport::charger(&lundis, fsport.as_deref()));
        let m = t.spawn(|| memoire::charger(&lundis, mexe.as_deref()));
        (d.join().unwrap_or_default(), j.join().unwrap_or_default(), sp.join().unwrap_or_default(), m.join().unwrap_or_default())
    });
    lundis
        .iter()
        .enumerate()
        .map(|(i, l)| Semaine {
            lundi: *l,
            dev: d.get(i).cloned().flatten(),
            jeu: j.get(i).cloned().flatten(),
            sport: sp.get(i).cloned().flatten(),
            memoire: m.get(i).cloned().flatten(),
        })
        .collect()
}

enum Msg {
    Charge(Vec<Semaine>, u128),
    Maj(Option<socle::update::Release>),
    Installe(Result<(), String>),
}

pub struct App {
    pub ctx: egui::Context,
    pub s: Settings,
    pub pal: Palette,
    saver: socle::settings::Saver,
    semaines: Vec<Semaine>,
    /// Semaine affichée (index dans `semaines`).
    i: usize,
    charge: bool,
    ms: u128,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    toasts: Toasts,
    palette: socle::ui::Palette,
    aide: bool,
    maj: Option<socle::update::Release>,
    pub quitter: bool,
}

impl App {
    pub fn new(ctx: &egui::Context, s: Settings) -> Self {
        let dark = s.dark.unwrap_or_else(|| theme::system_dark(ctx));
        let pal = theme::palette(dark);
        theme::install_fonts(ctx);
        theme::apply(ctx, &pal);
        let (tx, rx) = channel();
        let mut app = App {
            ctx: ctx.clone(),
            s,
            pal,
            saver: socle::settings::Saver::new(std::time::Duration::from_millis(500)),
            semaines: Vec::new(),
            i: 0,
            charge: false,
            ms: 0,
            tx,
            rx,
            toasts: Toasts::default(),
            palette: socle::ui::Palette::default(),
            aide: false,
            maj: None,
            quitter: false,
        };
        if let Some(msg) = socle::crash::pending(APP) {
            app.toasts.error(msg);
        }
        app.recharger();
        if socle::update::due(app.s.last_update_check) && !socle::testmode::enabled() {
            app.s.last_update_check = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            app.saver.touch();
            let (tx, c) = (app.tx.clone(), ctx.clone());
            std::thread::spawn(move || {
                let _ = tx.send(Msg::Maj(UPDATER.check().ok().flatten()));
                c.request_repaint();
            });
        }
        app
    }

    fn recharger(&mut self) {
        let (tx, c, s) = (self.tx.clone(), self.ctx.clone(), self.s.clone());
        std::thread::spawn(move || {
            let t = std::time::Instant::now();
            let v = std::panic::catch_unwind(|| charger(&s)).unwrap_or_default();
            let _ = tx.send(Msg::Charge(v, t.elapsed().as_millis()));
            c.request_repaint();
        });
    }

    pub fn tick(&mut self) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Charge(v, ms) => {
                    let premier = !self.charge;
                    self.semaines = v;
                    self.charge = true;
                    self.ms = ms;
                    if premier {
                        let d = derniere_semaine();
                        self.i = self.semaines.iter().position(|w| w.lundi == d).unwrap_or(self.semaines.len().saturating_sub(1));
                    }
                }
                Msg::Maj(r) => {
                    if let Some(r) = &r {
                        self.toasts.info(format!("Bilan {} est disponible (menu ⋯)", r.version));
                    }
                    self.maj = r;
                }
                Msg::Installe(Err(e)) => self.toasts.error(format!("Mise à jour impossible : {e}")),
                Msg::Installe(Ok(())) => self.quitter = true,
            }
        }
        // Le bilan de la semaine écoulée est « vu » dès qu'il est affiché.
        if let Some(w) = self.semaines.get(self.i) {
            let l = w.lundi.to_string();
            if w.lundi == derniere_semaine() && self.s.dernier_vu.as_deref() != Some(&l) {
                self.s.dernier_vu = Some(l);
                self.saver.touch();
            }
        }
        if let Some(d) = self.saver.tick(APP, &self.s) {
            self.ctx.request_repaint_after(d);
        }
        if let Some(e) = self.saver.take_error() {
            self.toasts.error(format!("Réglages non enregistrés : {e}"));
        }
    }

    pub fn pret(&self) -> bool {
        self.charge
    }

    pub fn flush(&mut self, size: Option<(f32, f32)>) {
        if size.is_some() {
            self.s.window = size;
        }
        let _ = self.saver.flush(APP, &self.s);
    }

    fn set_dark(&mut self, d: bool) {
        self.s.dark = Some(d);
        self.pal = theme::palette(d);
        theme::apply(&self.ctx, &self.pal);
        self.saver.touch();
    }

    fn set_auto(&mut self, on: bool) {
        let r = if on { crate::tache::activer() } else { crate::tache::desactiver() };
        match r {
            Ok(()) => {
                self.s.auto = on;
                self.saver.touch();
                self.toasts.ok(if on { "Bilan s'ouvrira le lundi" } else { "Ouverture du lundi désactivée" });
            }
            Err(e) if !on => {
                // La tâche n'existait déjà plus : le réglage suit quand même.
                self.s.auto = false;
                self.saver.touch();
                self.toasts.info(e);
            }
            Err(e) => self.toasts.error(e),
        }
    }

    fn commandes(&self) -> Vec<Command> {
        vec![
            Command::new("prec", "Semaine précédente").keys("←").icon(Icon::ArrowLeft),
            Command::new("suiv", "Semaine suivante").keys("→").icon(Icon::ArrowRight),
            Command::new("recharger", "Recharger les sources").keys("F5").icon(Icon::Refresh),
            Command::new("theme", "Basculer le thème clair / sombre").icon(if self.pal.dark { Icon::Sun } else { Icon::Moon }),
            Command::new("auto", if self.s.auto { "Ne plus ouvrir Bilan le lundi" } else { "Ouvrir Bilan chaque lundi" }).icon(Icon::Clock),
            Command::new("sport", "Ouvrir le dossier d'export sport").icon(Icon::Folder),
            Command::new("maj", "Installer la mise à jour").icon(Icon::Download).enabled(self.maj.is_some()),
            Command::new("palette", "Palette de commandes").keys("Ctrl+K").icon(Icon::Command),
            Command::new("aide", "Raccourcis").keys("F1").icon(Icon::Help),
        ]
    }

    fn lancer(&mut self, id: &str) {
        match id {
            "prec" => self.i = self.i.saturating_sub(1),
            "suiv" => self.i = (self.i + 1).min(self.semaines.len().saturating_sub(1)),
            "recharger" => self.recharger(),
            "theme" => self.set_dark(!self.pal.dark),
            "auto" => self.set_auto(!self.s.auto),
            "sport" => {
                let d = socle::paths::config_dir(APP).join("sport");
                let _ = std::fs::create_dir_all(&d);
                let _ = std::process::Command::new("explorer.exe").arg(&d).spawn();
            }
            "maj" => {
                if let Some(r) = self.maj.clone() {
                    self.toasts.info("Téléchargement de la mise à jour…");
                    let (tx, c) = (self.tx.clone(), self.ctx.clone());
                    std::thread::spawn(move || {
                        let res = UPDATER.install(&r, &|_| {}).and_then(|exe| socle::update::relaunch(&exe));
                        let _ = tx.send(Msg::Installe(res));
                        c.request_repaint();
                    });
                }
            }
            "palette" => self.palette.open(),
            "aide" => self.aide = true,
            _ => {}
        }
    }

    pub fn handle_keys(&mut self) {
        let ctx = self.ctx.clone();
        if self.palette.handle_shortcut(&ctx) || self.palette.is_open() || self.aide {
            return;
        }
        let k = |key| ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
        if k(egui::Key::ArrowLeft) {
            self.lancer("prec");
        }
        if k(egui::Key::ArrowRight) {
            self.lancer("suiv");
        }
        if k(egui::Key::F5) {
            self.lancer("recharger");
        }
        if k(egui::Key::F1) {
            self.aide = true;
        }
    }

    pub fn draw(&mut self, ui: &mut Ui) {
        let p = self.pal;
        let cmds = self.commandes();
        let mut faire: Option<String> = None;

        egui::Panel::top("barre").frame(Frame::new().fill(p.surface).inner_margin(Margin::symmetric(12, 8))).show(ui, |ui| {
            ui.horizontal(|ui| {
                let n = self.semaines.len();
                if ui.add_enabled_ui(self.i > 0, |ui| icons::button(ui, Icon::ArrowLeft, 32.0, Look::tool(&p), "Semaine précédente (←)").clicked()).inner {
                    faire = Some("prec".into());
                }
                if ui.add_enabled_ui(self.i + 1 < n, |ui| icons::button(ui, Icon::ArrowRight, 32.0, Look::tool(&p), "Semaine suivante (→)").clicked()).inner {
                    faire = Some("suiv".into());
                }
                let titre = self.semaines.get(self.i).map_or("Bilan".to_string(), |w| titre_semaine(w.lundi));
                ui.label(RichText::new(titre).size(taille::TITRE).color(p.text));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.menu_button(RichText::new("⋯").size(18.0), |ui| {
                        let mut auto = self.s.auto;
                        if ui.checkbox(&mut auto, "Ouvrir Bilan chaque lundi").changed() {
                            faire = Some("auto".into());
                        }
                        if ui.button("Dossier d'export sport…").clicked() {
                            faire = Some("sport".into());
                        }
                        if ui.button("Recharger (F5)").clicked() {
                            faire = Some("recharger".into());
                        }
                        if let Some(r) = &self.maj {
                            if ui.button(format!("Mettre à jour vers {}", r.version)).clicked() {
                                faire = Some("maj".into());
                            }
                        }
                        ui.separator();
                        ui.label(RichText::new(format!("Bilan {} · socle {}", env!("CARGO_PKG_VERSION"), socle::VERSION)).size(taille::META).color(p.muted));
                    });
                    let (ic, tip) = if p.dark { (Icon::Sun, "Thème clair") } else { (Icon::Moon, "Thème sombre") };
                    if icons::button(ui, ic, 32.0, Look::tool(&p), tip).clicked() {
                        faire = Some("theme".into());
                    }
                    if self.charge {
                        ui.label(RichText::new(format!("{} ms", self.ms)).size(taille::META).color(p.muted));
                    }
                });
            });
        });

        egui::CentralPanel::default().frame(Frame::new().fill(p.bg)).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let w = ui.available_width().min(760.0);
                let marge = ((ui.available_width() - w) / 2.0).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(marge);
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        ui.add_space(12.0);
                        if !self.charge {
                            ui.label(RichText::new("Lecture des sources…").color(p.muted));
                        } else if let Some(s) = self.semaines.get(self.i).cloned() {
                            self.page(ui, &s);
                        }
                        ui.add_space(16.0);
                    });
                });
            });
        });

        self.toasts.ui(ui, &p);
        if let Some(id) = self.palette.show(&self.ctx, &p, &cmds) {
            faire = Some(id);
        }
        socle::ui::help_window(&self.ctx, &p, &mut self.aide, &cmds);
        if let Some(id) = faire {
            self.lancer(&id);
        }
    }

    fn page(&self, ui: &mut Ui, s: &Semaine) {
        let p = self.pal;
        if s.lundi > derniere_semaine() {
            ui.label(RichText::new("Semaine en cours : chiffres provisoires.").size(taille::META).color(p.muted));
            ui.add_space(6.0);
        }
        let c = constats(&self.semaines, self.i);
        if !c.is_empty() {
            carte(ui, &p, "CONSTATS", |ui| {
                for l in c {
                    ui.label(RichText::new(format!("•  {l}")).size(taille::TEXTE).color(p.text));
                }
            });
        }

        carte(ui, &p, "SPORT", |ui| match &s.sport {
            None => {
                vide(ui, &p, "Pas de données pour cette semaine. Dans l'appli sport, touche « Exporter », puis dépose le fichier dans Téléchargements (ou le dossier d'export sport du menu ⋯).");
            }
            Some(sp) => {
                chiffres(ui, &p, &[(sp.seances.to_string(), "séances"), (format!("{} kg", un(sp.volume_kg)), "volume (charge × reps)")]);
                barres(ui, &p, &sp.par_jour, p.add);
                ui.add_space(6.0);
                for (nom, n, obj) in &sp.muscles {
                    jauge(ui, &p, nom, *n, *obj);
                }
                for (nom, a, b) in &sp.progres {
                    ui.label(RichText::new(format!("↑ {nom} : {} → {} kg", un(*a), un(*b))).size(taille::META).color(p.add));
                }
            }
        });

        carte(ui, &p, "JEU", |ui| match &s.jeu {
            None => vide(ui, &p, "Pas assez de relevés d'Ultimate pour estimer cette semaine."),
            Some(j) => {
                chiffres(ui, &p, &[(format!("≈ {} h", un(j.minutes as f64 / 60.0)), "de jeu")]);
                for (nom, m) in &j.top {
                    ligne(ui, &p, nom, &format!("{} h", un(*m as f64 / 60.0)));
                }
                ui.label(RichText::new("Estimation : écarts entre les relevés de la bibliothèque d'Ultimate (pas des sessions exactes).").size(taille::META).color(p.muted));
            }
        });

        carte(ui, &p, "DEV", |ui| match &s.dev {
            None => vide(ui, &p, "Git introuvable ou aucun dépôt sous C:\\Dev."),
            Some(d) => {
                let actifs = d.par_jour.iter().filter(|n| **n > 0).count();
                chiffres(ui, &p, &[(d.commits.to_string(), "commits"), (actifs.to_string(), "jours actifs")]);
                barres(ui, &p, &d.par_jour, p.accent);
                for (nom, n) in d.projets.iter().take(6) {
                    ligne(ui, &p, nom, &n.to_string());
                }
            }
        });

        if let Some(m) = &s.memoire {
            carte(ui, &p, "PRESSE-PAPIERS ET CAPTURES", |ui| {
                chiffres(ui, &p, &[(m.copies.to_string(), "copies"), (m.captures.to_string(), "captures")]);
                barres(ui, &p, &m.par_jour, p.live);
            });
        }
    }
}

fn titre_semaine(l: NaiveDate) -> String {
    const MOIS: [&str; 12] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
    use chrono::Datelike;
    let f = l + Duration::days(6);
    let m = |d: NaiveDate| MOIS[d.month0() as usize];
    if l.month() == f.month() {
        format!("Semaine du {} au {} {}", l.day(), f.day(), m(f))
    } else {
        format!("Semaine du {} {} au {} {}", l.day(), m(l), f.day(), m(f))
    }
}

fn carte(ui: &mut Ui, p: &Palette, titre: &str, add: impl FnOnce(&mut Ui)) {
    Frame::new().fill(p.surface).corner_radius(CornerRadius::same(theme::arrondi::CARTE)).inner_margin(Margin::same(14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(titre).size(taille::ENTETE).color(p.muted));
        ui.add_space(4.0);
        add(ui);
    });
    ui.add_space(10.0);
}

fn vide(ui: &mut Ui, p: &Palette, t: &str) {
    ui.label(RichText::new(t).size(taille::TEXTE).color(p.muted));
}

fn chiffres(ui: &mut Ui, p: &Palette, v: &[(String, &str)]) {
    ui.horizontal(|ui| {
        for (n, l) in v {
            ui.label(RichText::new(n).size(24.0).color(p.text));
            ui.label(RichText::new(*l).size(taille::META).color(p.muted));
            ui.add_space(18.0);
        }
    });
}

fn ligne(ui: &mut Ui, p: &Palette, a: &str, b: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(a).size(taille::TEXTE).color(p.text));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.label(RichText::new(b).size(taille::TEXTE).color(p.muted)));
    });
}

/// Sept barres, lundi → dimanche.
fn barres(ui: &mut Ui, p: &Palette, v: &[u32; 7], c: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(7.0 * 34.0, 64.0), Sense::hover());
    let max = (*v.iter().max().unwrap_or(&0)).max(1) as f32;
    let pt = ui.painter();
    for (i, n) in v.iter().enumerate() {
        let x = r.left() + i as f32 * 34.0;
        let h = 44.0 * *n as f32 / max;
        let bas = r.top() + 46.0;
        pt.rect_filled(egui::Rect::from_min_max(egui::pos2(x + 6.0, bas - h.max(2.0)), egui::pos2(x + 28.0, bas)), 3.0, if *n > 0 { c } else { p.border });
        pt.text(egui::pos2(x + 17.0, bas + 9.0), egui::Align2::CENTER_CENTER, ["L", "M", "M", "J", "V", "S", "D"][i], FontId::proportional(taille::META), p.muted);
    }
}

fn jauge(ui: &mut Ui, p: &Palette, nom: &str, n: u32, obj: u32) {
    ui.horizontal(|ui| {
        ui.add_sized(vec2(130.0, 18.0), egui::Label::new(RichText::new(nom).size(taille::META).color(p.text)));
        let (r, _) = ui.allocate_exact_size(vec2(180.0, 8.0), Sense::hover());
        let pt = ui.painter();
        pt.rect_filled(r, 4.0, p.soft);
        let f = if obj == 0 { 1.0 } else { (n as f32 / obj as f32).min(1.0) };
        let mut r2 = r;
        r2.set_width(r.width() * f);
        pt.rect_filled(r2, 4.0, if n >= obj { p.add } else { p.live });
        ui.label(RichText::new(format!("{n}/{obj}")).size(taille::META).color(p.muted));
    });
}
