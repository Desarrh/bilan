//! Source Mémoire : `Memoire.exe --stats-semaine --json --semaine AAAA-MM-JJ`, appelé sans fenêtre.

use super::{commande, sortie};
use crate::modele::Memoire;
use chrono::NaiveDate;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Deserialize)]
struct Stats {
    v: u32,
    copies: u32,
    captures: u32,
    par_jour: [u32; 7],
}

/// Chemin réglé s'il existe, sinon les emplacements connus.
pub fn trouver(reglage: Option<&str>) -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = reglage.filter(|r| !r.trim().is_empty()).map(PathBuf::from).into_iter().collect();
    c.push(PathBuf::from(r"C:\Dev\memoire\dist\Memoire.exe"));
    if let Some(p) = std::env::var_os("LOCALAPPDATA") {
        c.push(PathBuf::from(p).join("Memoire").join("Memoire.exe"));
    }
    c.push(PathBuf::from(r"C:\Dev\memoire\target\release\memoire.exe"));
    // Un chemin réglé qui n'existe pas n'empêche pas de retomber sur les emplacements connus.
    c.into_iter().find(|p| p.is_file())
}

/// Une entrée par lundi, appels en parallèle, 3 s max chacun ; None si absent/échec/JSON invalide.
pub fn charger(lundis: &[NaiveDate], exe: Option<&Path>) -> Vec<Option<Memoire>> {
    let Some(exe) = exe.and_then(|e| e.to_str()) else { return vec![None; lundis.len()] };
    std::thread::scope(|sc| {
        let taches: Vec<_> = lundis
            .iter()
            .map(|l| {
                sc.spawn(move || {
                    let mut c = commande(exe);
                    c.args(["--stats-semaine", "--json", "--semaine", &l.format("%Y-%m-%d").to_string()]);
                    analyser(&sortie(c, Duration::from_secs(3))?)
                })
            })
            .collect();
        taches.into_iter().map(|t| t.join().ok().flatten()).collect()
    })
}

/// Analyse la sortie JSON de Mémoire (version 1 seulement).
fn analyser(octets: &[u8]) -> Option<Memoire> {
    let s: Stats = serde_json::from_slice(octets).ok()?;
    (s.v == 1).then_some(Memoire { copies: s.copies, captures: s.captures, par_jour: s.par_jour })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyse_json() {
        let ok = br#"{"v":1,"debut":"2026-11-16","copies":42,"captures":3,"par_jour":[1,2,3,4,5,6,7]}"#;
        let m = analyser(ok).unwrap();
        assert_eq!((m.copies, m.captures, m.par_jour), (42, 3, [1, 2, 3, 4, 5, 6, 7]));
        assert!(analyser(br#"{"v":2,"copies":0,"captures":0,"par_jour":[0,0,0,0,0,0,0]}"#).is_none());
        assert!(analyser(br#"{"v":1,"copies":0,"captures":0,"par_jour":[0]}"#).is_none());
        assert!(analyser(b"pas du json").is_none());
    }

    #[test]
    fn sans_exe() {
        let l = NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
        assert_eq!(charger(&[l, l], None), vec![None, None]);
        assert!(trouver(Some(r"C:\n\existe\pas.exe")).is_none_or(|p| p.is_file()));
    }
}
