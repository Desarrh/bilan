//! Source sport : export JSON de l'appli sport (`appli-sport-<jour>.json`), lu en lecture seule.

use crate::modele::{Sport, bornes, index_jour};
use chrono::{DateTime, Duration, Local, NaiveDate};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Muscles dans l'ordre du catalogue de l'appli sport : (id, nom).
const MUSCLES: &[(&str, &str)] = &[
    ("dos", "Dos"),
    ("biceps", "Biceps"),
    ("avant_bras", "Avant-bras"),
    ("pectoraux", "Pectoraux"),
    ("triceps", "Triceps"),
    ("epaules", "Épaules"),
    ("quadriceps", "Quadriceps"),
    ("ischio_jambiers", "Ischio-jambiers"),
    ("adducteurs", "Adducteurs"),
    ("fessiers", "Fessiers"),
    ("mollets", "Mollets"),
    ("abdos", "Abdos"),
];

const M: char = 'm'; // musculation
const B: char = 'b'; // bras de fer
const E: char = 'e'; // étirement

/// Exercices : (id, nom, muscle principal, catégorie, unilatéral).
const CATALOGUE: &[(&str, &str, Option<&str>, char, bool)] = &[
    ("tirage_vertical", "Tirage vertical", Some("dos"), M, false),
    ("tirage_horizontal", "Tirage horizontal", Some("dos"), M, false),
    ("half_curl_table", "Half curl sur table", Some("biceps"), M, true),
    ("curl_strict", "Curl strict", Some("biceps"), M, false),
    ("pec_fly", "Écarté pectoraux", Some("pectoraux"), M, false),
    ("pec_press", "Développé machine", Some("pectoraux"), M, false),
    ("triceps_overhead", "Triceps au-dessus de la tête", Some("triceps"), M, true),
    ("triceps_pushdown", "Triceps vers le bas", Some("triceps"), M, true),
    ("elevation_laterale", "Élévation latérale poulie", Some("epaules"), M, true),
    ("leg_press", "Presse à cuisses", Some("quadriceps"), M, true),
    ("leg_curl_assis", "Leg curl assis", Some("ischio_jambiers"), M, false),
    ("adducteurs_assis", "Adducteurs assis", Some("adducteurs"), M, false),
    ("hip_thrust", "Hip thrust", Some("fessiers"), M, false),
    ("mollets_debout", "Mollets debout", Some("mollets"), M, false),
    ("crunch_poulie", "Crunch à la poulie", Some("abdos"), M, false),
    ("wrist_curl", "Wrist curl", Some("avant_bras"), M, true),
    ("pronation_iso", "Pronation haute (iso)", Some("avant_bras"), M, true),
    ("rising", "Rising", Some("avant_bras"), M, true),
    ("doigts", "Doigts", Some("avant_bras"), M, true),
    ("extension_poignet", "Extension poignet et doigts", Some("avant_bras"), M, true),
    ("back_pressure", "Back pressure", None, B, true),
    ("side_pressure", "Side pressure", None, B, true),
    ("sparring", "Sparring", None, B, false),
    ("flexion_avant_debout", "Flexion avant debout", Some("ischio_jambiers"), E, false),
    ("ischios_allonge_pnf", "Ischio-jambiers allongé (PNF)", Some("ischio_jambiers"), E, false),
    ("mollets_mur", "Mollets au mur", Some("mollets"), E, false),
    ("fessiers_figure_4", "Fessiers en « 4 »", Some("fessiers"), E, false),
    ("rotation_externe", "Rotation externe à la poulie", Some("epaules"), M, true),
    ("face_pull", "Face pull", Some("epaules"), M, false),
    ("elevation_t", "Élévation en T allongé", Some("epaules"), M, false),
    ("sdt_roumain", "Soulevé de terre roumain profond", Some("ischio_jambiers"), E, false),
    ("jefferson_curl", "Jefferson curl", Some("dos"), E, false),
    ("glissement_mur", "Glissement au mur (wall angel)", Some("epaules"), E, false),
    ("etirement_pectoral", "Étirement du pectoral", Some("pectoraux"), E, false),
    ("retraction_cervicale", "Rétraction cervicale", Some("dos"), E, false),
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Serie {
    exercice_id: String,
    horodatage: String,
    cote: Option<String>,
    charge_kg: Option<f64>,
    reps: Option<f64>,
}

#[derive(Deserialize)]
struct Etat {
    #[serde(default)]
    series: Vec<Serie>,
    #[serde(default)]
    objectifs: HashMap<String, u32>,
}

/// Le fichier appli-sport-*.json le plus récent (date de modification) parmi ces dossiers.
pub fn trouver(dossiers: &[PathBuf]) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for d in dossiers {
        let Ok(rd) = std::fs::read_dir(d) else { continue };
        for e in rd.flatten() {
            let nom = e.file_name().to_string_lossy().to_lowercase();
            if !(nom.starts_with("appli-sport-") && nom.ends_with(".json")) {
                continue;
            }
            let Ok(t) = e.metadata().and_then(|m| m.modified()) else { continue };
            if best.as_ref().is_none_or(|(b, _)| t > *b) {
                best = Some((t, e.path()));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Dossiers par défaut : %USERPROFILE%\Downloads et %APPDATA%\Bilan\sport.
pub fn dossiers() -> Vec<PathBuf> {
    let var = |k| std::env::var_os(k).map(PathBuf::from);
    let mut v = Vec::new();
    if let Some(p) = var("USERPROFILE") {
        v.push(p.join("Downloads"));
    }
    if let Some(p) = var("APPDATA") {
        v.push(p.join("Bilan").join("sport"));
    }
    v
}

/// Une entrée par lundi. None si pas de fichier, ou si la semaine est après la date de modification
/// du fichier (pas encore exportée) ou entièrement avant la première série. Sinon Some (0 séance est une vraie donnée).
pub fn charger(lundis: &[NaiveDate], fichier: Option<&Path>) -> Vec<Option<Sport>> {
    let vide = || vec![None; lundis.len()];
    let Some(f) = fichier else { return vide() };
    let Ok(txt) = std::fs::read_to_string(f) else { return vide() };
    let Ok(etat) = serde_json::from_str::<Etat>(&txt) else { return vide() };
    let Some(modif) = std::fs::metadata(f)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
    else {
        return vide();
    };
    let premiere = etat
        .series
        .iter()
        .filter_map(|s| DateTime::parse_from_rfc3339(&s.horodatage).ok())
        .map(|d| d.timestamp())
        .min();
    lundis
        .iter()
        .map(|&l| {
            let (debut, fin) = bornes(l);
            if debut > modif || premiere.is_none_or(|p| fin <= p) {
                return None;
            }
            Some(calculer(&etat, l))
        })
        .collect()
}

/// Statistiques de la semaine commençant au lundi `l` (fonction pure).
fn calculer(etat: &Etat, l: NaiveDate) -> Sport {
    // Charge max par exercice : semaine courante, puis 4 semaines précédentes.
    let mut courante: HashMap<&str, f64> = HashMap::new();
    let mut avant: HashMap<&str, f64> = HashMap::new();
    // Par exercice, séries de la semaine : (total, gauche, droite).
    let mut comptes: HashMap<&str, (u32, u32, u32)> = HashMap::new();
    let mut jours = HashSet::new();
    let mut s = Sport::default();
    for x in &etat.series {
        let Ok(t) = DateTime::parse_from_rfc3339(&x.horodatage) else { continue };
        let jour = t.with_timezone(&Local).date_naive();
        let id = x.exercice_id.as_str();
        let charge = x.charge_kg.unwrap_or(0.0);
        if let Some(i) = index_jour(l, jour) {
            s.par_jour[i] += 1;
            jours.insert(jour);
            let c = comptes.entry(id).or_default();
            c.0 += 1;
            match x.cote.as_deref() {
                Some("G") => c.1 += 1,
                Some("D") => c.2 += 1,
                _ => {}
            }
            if CATALOGUE.iter().any(|e| e.0 == id && e.3 == M) {
                s.volume_kg += charge * x.reps.unwrap_or(0.0);
            }
            let m = courante.entry(id).or_insert(0.0);
            *m = m.max(charge);
        } else if jour < l && jour >= l - Duration::days(28) {
            let m = avant.entry(id).or_insert(0.0);
            *m = m.max(charge);
        }
    }
    s.seances = jours.len() as u32;
    s.muscles = MUSCLES
        .iter()
        .map(|&(mid, nom)| {
            let n = CATALOGUE
                .iter()
                .filter(|e| e.3 == M && e.2 == Some(mid))
                .filter_map(|e| comptes.get(e.0).map(|c| if e.4 { c.1.max(c.2) } else { c.0 }))
                .sum();
            (nom.to_string(), n, etat.objectifs.get(mid).copied().unwrap_or(4))
        })
        .collect();
    for (id, &now) in &courante {
        // Progrès seulement s'il existe une charge antérieure à dépasser.
        if let Some(&av) = avant.get(id).filter(|&&a| a > 0.0) {
            if now > av {
                let nom = CATALOGUE.iter().find(|e| e.0 == *id).map_or(*id, |e| e.1);
                s.progres.push((nom.to_string(), av, now));
            }
        }
    }
    s.progres.sort_by(|a, b| a.0.cmp(&b.0));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serie(id: &str, jour: &str, cote: Option<&str>, charge: f64, reps: f64) -> Serie {
        Serie {
            exercice_id: id.into(),
            horodatage: format!("{jour}T12:00:00.000Z"),
            cote: cote.map(Into::into),
            charge_kg: Some(charge),
            reps: Some(reps),
        }
    }

    fn lundi() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
    }

    fn etat() -> Etat {
        Etat {
            series: vec![
                serie("half_curl_table", "2026-11-17", Some("G"), 10.0, 8.0),
                serie("half_curl_table", "2026-11-17", Some("D"), 10.0, 8.0),
                serie("half_curl_table", "2026-11-17", Some("G"), 12.0, 8.0),
                serie("tirage_vertical", "2026-11-19", None, 50.0, 10.0),
                serie("tirage_vertical", "2026-11-19", None, 50.0, 10.0),
                serie("jefferson_curl", "2026-11-19", None, 20.0, 5.0),
                serie("tirage_vertical", "2026-11-05", None, 40.0, 10.0),
            ],
            objectifs: HashMap::from([("dos".to_string(), 10)]),
        }
    }

    #[test]
    fn calcul_semaine() {
        let s = calculer(&etat(), lundi());
        assert_eq!(s.seances, 2);
        assert_eq!(s.par_jour, [0, 3, 0, 3, 0, 0, 0]);
        let m = |n: &str| s.muscles.iter().find(|x| x.0 == n).cloned().unwrap();
        assert_eq!(m("Biceps"), ("Biceps".into(), 2, 4)); // max(G=2, D=1)
        assert_eq!(m("Dos"), ("Dos".into(), 2, 10)); // l'étirement ne compte pas
        assert_eq!(s.muscles.len(), 12);
        assert_eq!(s.volume_kg, 256.0 + 1000.0);
        assert_eq!(s.progres, vec![("Tirage vertical".to_string(), 40.0, 50.0)]);
    }

    #[test]
    fn charger_fichier() {
        let dir = std::env::temp_dir().join(format!("bilan-sport-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("appli-sport-2026-11-20.json");
        let json = r#"{"version":1,"series":[{"id":"a","exerciceId":"curl_strict","horodatage":"2026-11-17T12:00:00.000Z","regime":"concentrique","cote":null,"chargeKg":10,"reps":8,"dureeS":null,"note":null}],"favoris":[],"objectifs":{}}"#;
        std::fs::write(&f, json).unwrap();
        assert_eq!(trouver(std::slice::from_ref(&dir)), Some(f.clone()));
        let avant = lundi() - Duration::days(14); // entièrement avant la première série
        let futur = NaiveDate::from_ymd_opt(2999, 1, 4).unwrap(); // après l'export
        let r = charger(&[avant, lundi(), futur], Some(&f));
        assert!(r[0].is_none() && r[2].is_none());
        let s = r[1].clone().unwrap();
        assert_eq!((s.seances, s.volume_kg), (1, 80.0));
        assert_eq!(charger(&[lundi()], None), vec![None]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
