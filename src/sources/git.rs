//! Commits de la semaine, lus avec `git log` dans les dépôts des racines configurées.

use super::{commande, sortie};
use crate::modele::{Dev, index_jour, jour_local, minuit};
use chrono::NaiveDate;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct Reglages<'a> {
    pub racines: &'a [String],
    pub auteur: &'a str,
    pub exclure: &'a [String],
}

/// Une entrée par lundi (même ordre). None partout si git est absent ou aucun dépôt trouvé ;
/// sinon Some(Dev) pour chaque semaine (0 commit = Some avec zéros).
pub fn charger(lundis: &[NaiveDate], r: &Reglages) -> Vec<Option<Dev>> {
    let Some(&premier) = lundis.iter().min() else { return Vec::new() };
    let depots: Vec<PathBuf> = r.racines.iter().flat_map(|x| depots(Path::new(x), r.exclure)).collect();
    if depots.is_empty() {
        return vec![None; lundis.len()];
    }
    let depuis = minuit(premier);
    let resultats: Vec<Option<Vec<(String, i64)>>> = std::thread::scope(|s| {
        let h: Vec<_> = depots.iter().map(|d| s.spawn(move || log(d, r.auteur, depuis))).collect();
        h.into_iter().map(|h| h.join().ok().flatten()).collect()
    });
    // Aucun `git log` n'a répondu : git absent (ou tous les dépôts illisibles).
    if resultats.iter().all(Option::is_none) {
        return vec![None; lundis.len()];
    }
    let commits: Vec<(String, i64)> = resultats.into_iter().flatten().flatten().collect();
    regrouper(lundis, &commits).into_iter().map(Some).collect()
}

/// Horodatages des commits d'un dépôt depuis `depuis`, étiquetés du nom du dépôt.
fn log(depot: &Path, auteur: &str, depuis: i64) -> Option<Vec<(String, i64)>> {
    let mut c = commande("git");
    c.arg("-C").arg(depot).args(["log", "--all", "--no-merges", "--format=%ct"]);
    c.arg(format!("--since={depuis}"));
    if !auteur.is_empty() {
        c.arg(format!("--author={auteur}"));
    }
    let nom = depot.file_name()?.to_string_lossy().into_owned();
    let out = sortie(c, Duration::from_secs(10))?;
    let txt = String::from_utf8_lossy(&out);
    Some(txt.lines().filter_map(|l| l.trim().parse().ok()).map(|t| (nom.clone(), t)).collect())
}

/// Dépôts (dossier `.git`, pas les worktrees) dans la racine et ses sous-dossiers sur 2 niveaux.
fn depots(racine: &Path, exclure: &[String]) -> Vec<PathBuf> {
    let mut v = Vec::new();
    explorer(racine, 0, exclure, &mut v);
    v
}

fn explorer(d: &Path, niveau: u32, exclure: &[String], v: &mut Vec<PathBuf>) {
    if d.join(".git").is_dir() {
        v.push(d.to_path_buf());
    }
    if niveau >= 2 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(d) else { return };
    for e in rd.flatten() {
        let nom = e.file_name().to_string_lossy().to_lowercase();
        let saute = nom.starts_with('.')
            || matches!(nom.as_str(), "node_modules" | "target")
            || exclure.iter().any(|x| !x.is_empty() && nom.starts_with(&x.to_lowercase()));
        if !saute && e.path().is_dir() {
            explorer(&e.path(), niveau + 1, exclure, v);
        }
    }
}

/// Répartit les commits (nom du dépôt, instant Unix) par semaine.
fn regrouper(lundis: &[NaiveDate], commits: &[(String, i64)]) -> Vec<Dev> {
    lundis
        .iter()
        .map(|&l| {
            let mut dev = Dev::default();
            let mut par_depot: HashMap<&str, u32> = HashMap::new();
            for (nom, t) in commits {
                if let Some(i) = index_jour(l, jour_local(*t)) {
                    dev.commits += 1;
                    dev.par_jour[i] += 1;
                    *par_depot.entry(nom).or_default() += 1;
                }
            }
            dev.projets = par_depot.into_iter().map(|(n, c)| (n.to_string(), c)).collect();
            dev.projets.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            dev
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regroupe_par_semaine_jour_et_projet() {
        let l = NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
        let l2 = l + chrono::Duration::days(7);
        let h = 3600;
        let c = |n: &str, t: i64| (n.to_string(), t);
        let commits = [
            c("a", minuit(l) + h),
            c("a", minuit(l) + 2 * h),
            c("b", minuit(l + chrono::Duration::days(6)) + h),
            c("b", minuit(l2) + h),
            c("b", minuit(l) - h), // semaine d'avant : ignoré
        ];
        let r = regrouper(&[l, l2], &commits);
        assert_eq!(r[0].commits, 3);
        assert_eq!(r[0].par_jour, [2, 0, 0, 0, 0, 0, 1]);
        assert_eq!(r[0].projets, vec![("a".into(), 2), ("b".into(), 1)]);
        assert_eq!(r[1].commits, 1);
        assert_eq!(r[1].par_jour[0], 1);
    }

    #[test]
    fn depots_ignore_worktrees_et_exclus() {
        let t = std::env::temp_dir().join(format!("bilan_git_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        let mk = |p: &str| std::fs::create_dir_all(t.join(p)).unwrap();
        mk("ok/.git");
        mk("Godot_x/.git");
        mk("wt");
        std::fs::write(t.join("wt/.git"), "gitdir: x").unwrap();
        mk("g/profond/.git");
        mk("g/node_modules/p/.git");
        let mut v: Vec<String> = depots(&t, &["godot".into()])
            .iter()
            .map(|p| p.strip_prefix(&t).unwrap().to_string_lossy().replace('\\', "/"))
            .collect();
        v.sort();
        let _ = std::fs::remove_dir_all(&t);
        assert_eq!(v, ["g/profond", "ok"]);
    }
}
