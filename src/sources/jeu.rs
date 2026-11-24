//! Temps de jeu estimé depuis les photos quotidiennes des cumuls du launcher Ultimate
//! (`history.json`) complétées par les nôtres, prises à chaque lancement de Bilan.

use crate::modele::Jeu;
use chrono::{Duration, NaiveDate};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

type Photos = BTreeMap<NaiveDate, HashMap<String, u32>>;

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Entree {
    id: String,
    name: String,
    playtime_min: u32,
}

pub fn dossier_ultimate() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default();
    base.join("dev.local.launcher")
}

/// Lit un fichier JSON de photos ({jour: {id: minutes}}) ; les jours illisibles sont ignorés.
fn lire_photos(p: &Path) -> Photos {
    let brut: BTreeMap<String, HashMap<String, u32>> =
        std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    brut.into_iter().filter_map(|(j, m)| Some((j.parse().ok()?, m))).collect()
}

fn lire_bibliotheque(dossier: &Path) -> Vec<Entree> {
    std::fs::read(dossier.join("library.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Photographie les cumuls actuels de library.json dans `photos` (clé = jour local),
/// seulement si quelque chose a changé ; garde 400 jours. Erreurs ignorées.
pub fn photographier(dossier_ultimate: &Path, photos: &Path, aujourdhui: NaiveDate) {
    let jour: HashMap<String, u32> = lire_bibliotheque(dossier_ultimate)
        .into_iter()
        .filter(|g| g.playtime_min > 0 && !g.id.is_empty())
        .map(|g| (g.id, g.playtime_min))
        .collect();
    if jour.is_empty() {
        return; // bibliothèque absente ou vide : on n'écrase rien
    }
    let mut p = lire_photos(photos);
    if p.get(&aujourdhui) == Some(&jour) {
        return;
    }
    p.insert(aujourdhui, jour);
    while p.len() > 400 {
        p.pop_first();
    }
    let sortie: BTreeMap<String, &HashMap<String, u32>> = p.iter().map(|(j, m)| (j.to_string(), m)).collect();
    if let Ok(b) = serde_json::to_vec(&sortie) {
        let _ = socle::paths::write_atomic(photos, &b);
    }
}

/// Une entrée par lundi. None si pas assez de photos pour estimer la semaine.
pub fn charger(lundis: &[NaiveDate], dossier_ultimate: &Path, photos: &Path) -> Vec<Option<Jeu>> {
    let mut tout = lire_photos(&dossier_ultimate.join("history.json"));
    for (j, m) in lire_photos(photos) {
        let e = tout.entry(j).or_default();
        for (id, v) in m {
            let c = e.entry(id).or_default();
            *c = (*c).max(v);
        }
    }
    let noms: HashMap<String, String> =
        lire_bibliotheque(dossier_ultimate).into_iter().map(|g| (g.id, g.name)).collect();
    lundis.iter().map(|&l| semaine(l, &tout, &noms)).collect()
}

/// Estimation d'une semaine [lundi, dimanche] à partir des cumuls photographiés.
fn semaine(lundi: NaiveDate, photos: &Photos, noms: &HashMap<String, String>) -> Option<Jeu> {
    let dimanche = lundi + Duration::days(6);
    let (jb, base) = photos
        .range(..lundi)
        .next_back()
        .or_else(|| photos.range(lundi..=dimanche).next())?;
    let (jf, fin) = photos.range(..=dimanche).next_back()?;
    if jf <= jb {
        return None;
    }
    let mut v: Vec<(String, u32)> = fin
        .iter()
        .map(|(id, f)| (id, f.saturating_sub(base.get(id).copied().unwrap_or(0))))
        .filter(|(_, m)| *m > 0)
        .map(|(id, m)| (noms.get(id).filter(|n| !n.is_empty()).unwrap_or(id).clone(), m))
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Some(Jeu { minutes: v.iter().map(|x| x.1).sum(), top: v.into_iter().take(3).collect() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, j).unwrap()
    }
    fn ph(l: &[(u32, &[(&str, u32)])]) -> Photos {
        l.iter().map(|(j, m)| (d(*j), m.iter().map(|(i, v)| (i.to_string(), *v)).collect())).collect()
    }

    #[test]
    fn calcul_de_semaine() {
        let noms: HashMap<String, String> = [("a".to_string(), "Alpha".to_string())].into();
        // Semaine du lundi 16 au dimanche 22.
        let p = ph(&[(10, &[("a", 100), ("b", 50)]), (18, &[("a", 160), ("b", 50)]), (21, &[("a", 190), ("b", 55), ("c", 7)]), (25, &[("a", 999)])]);
        let j = semaine(d(16), &p, &noms).unwrap();
        assert_eq!(j.minutes, 90 + 5 + 7);
        assert_eq!(j.top, vec![("Alpha".into(), 90), ("c".into(), 7), ("b".into(), 5)]);
        // Sans photo avant : base = première photo de la semaine.
        let p = ph(&[(18, &[("a", 160)]), (21, &[("a", 190)])]);
        assert_eq!(semaine(d(16), &p, &noms).unwrap().minutes, 30);
        // Une seule photo, ou aucune : pas d'estimation.
        assert_eq!(semaine(d(16), &ph(&[(18, &[("a", 1)])]), &noms), None);
        assert_eq!(semaine(d(16), &ph(&[(10, &[("a", 1)])]), &noms), None);
        assert_eq!(semaine(d(16), &ph(&[]), &noms), None);
    }

    #[test]
    fn photo_ecrite_une_fois() {
        let t = std::env::temp_dir().join(format!("bilan_jeu_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        std::fs::create_dir_all(&t).unwrap();
        std::fs::write(t.join("library.json"), r#"[{"id":"a","name":"Alpha","playtimeMin":30,"x":1},{"id":"z","playtimeMin":0}]"#).unwrap();
        let f = t.join("photos.json");
        photographier(&t, &f, d(20));
        let p = lire_photos(&f);
        assert_eq!(p[&d(20)], HashMap::from([("a".to_string(), 30)]));
        let _ = std::fs::remove_dir_all(&t);
    }
}
