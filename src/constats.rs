//! Constats factuels d'une semaine, comparée aux 4 précédentes et croisée entre sources.

use crate::modele::Semaine;

const JOURS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];

/// `semaines` va de la plus ancienne à la plus récente ; `i` est la semaine affichée.
pub fn constats(semaines: &[Semaine], i: usize) -> Vec<String> {
    let Some(s) = semaines.get(i) else { return Vec::new() };
    let avant = &semaines[i.saturating_sub(4)..i];
    let mut out = Vec::new();

    if let Some(sp) = &s.sport {
        let moy = moyenne(avant.iter().filter_map(|w| w.sport.as_ref().map(|x| x.seances as f64)));
        out.push(match moy {
            Some(m) => format!("{} séance{} de sport, contre {} en moyenne les semaines d'avant.", sp.seances, pluriel(sp.seances), un(m)),
            None => format!("{} séance{} de sport.", sp.seances, pluriel(sp.seances)),
        });
        let sous: Vec<String> = sp.muscles.iter().filter(|m| m.2 > 0 && m.1 < m.2).map(|m| format!("{} {}/{}", m.0, m.1, m.2)).collect();
        if !sous.is_empty() && sp.seances > 0 {
            out.push(format!("Sous l'objectif de séries : {}.", sous.join(", ")));
        }
        if !sp.progres.is_empty() {
            let p: Vec<String> = sp.progres.iter().take(3).map(|p| format!("{} ({} → {} kg)", p.0, un(p.1), un(p.2))).collect();
            out.push(format!("Charge en hausse : {}.", p.join(", ")));
        }
    }
    if let Some(j) = &s.jeu {
        let moy = moyenne(avant.iter().filter_map(|w| w.jeu.as_ref().map(|x| x.minutes as f64 / 60.0)));
        let h = j.minutes as f64 / 60.0;
        if let Some(m) = moy.filter(|m| *m > 0.5 && ((h - m) / m).abs() >= 0.25) {
            out.push(format!("Environ {} h de jeu, {} que d'habitude ({} h).", un(h), if h > m { "plus" } else { "moins" }, un(m)));
        }
    }
    if let Some(d) = &s.dev {
        let actifs = d.par_jour.iter().filter(|n| **n > 0).count();
        if d.commits > 0 {
            let (jour, _) = d.par_jour.iter().enumerate().max_by_key(|(_, n)| **n).unwrap_or((0, &0));
            out.push(format!("{} commit{} sur {} jour{}, surtout le {}.", d.commits, pluriel(d.commits), actifs, pluriel(actifs as u32), JOURS[jour]));
        }
    }

    // Croisements sur tout l'historique connu jusqu'à la semaine affichée.
    let passe = &semaines[..=i];
    if let Some(t) = croiser(passe, |w| w.sport.as_ref().map(|x| x.seances as f64)) {
        out.push(format!("Les semaines à plus de {} h de jeu : {} séance(s) en moyenne, contre {} les autres.", un(t.0), un(t.1), un(t.2)));
    }
    if let Some(t) = croiser(passe, |w| w.dev.as_ref().map(|x| x.commits as f64)) {
        out.push(format!("Les semaines à plus de {} h de jeu : {} commits en moyenne, contre {} les autres.", un(t.0), un(t.1), un(t.2)));
    }
    out
}

/// Sépare les semaines au-dessus et au-dessous de la médiane des heures de jeu et compare `f`.
/// Rend (seuil en heures, moyenne au-dessus, moyenne au-dessous) seulement si l'écart est net.
fn croiser(semaines: &[Semaine], f: impl Fn(&Semaine) -> Option<f64>) -> Option<(f64, f64, f64)> {
    let mut pts: Vec<(f64, f64)> = semaines.iter().filter_map(|w| Some((w.jeu.as_ref()?.minutes as f64 / 60.0, f(w)?))).collect();
    if pts.len() < 4 {
        return None;
    }
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let seuil = pts[pts.len() / 2 - 1].0;
    let haut = moyenne(pts.iter().filter(|p| p.0 > seuil).map(|p| p.1))?;
    let bas = moyenne(pts.iter().filter(|p| p.0 <= seuil).map(|p| p.1))?;
    let ecart = (haut - bas).abs();
    (ecart >= 1.0 && ecart >= 0.3 * haut.max(bas)).then_some((seuil, haut, bas))
}

fn moyenne(v: impl Iterator<Item = f64>) -> Option<f64> {
    let (s, n) = v.fold((0.0, 0), |(s, n), x| (s + x, n + 1));
    (n > 0).then(|| s / n as f64)
}

fn pluriel(n: u32) -> &'static str {
    if n > 1 { "s" } else { "" }
}

/// Nombre à une décimale au plus, virgule française.
pub fn un(x: f64) -> String {
    let r = (x * 10.0).round() / 10.0;
    if r.fract() == 0.0 { format!("{r:.0}") } else { format!("{r:.1}").replace('.', ",") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modele::{Jeu, Sport};

    #[test]
    fn croisement_jeu_sport() {
        let w = |h: u32, s: u32| Semaine {
            jeu: Some(Jeu { minutes: h * 60, top: vec![] }),
            sport: Some(Sport { seances: s, ..Default::default() }),
            ..Default::default()
        };
        let v = vec![w(20, 0), w(5, 3), w(18, 0), w(4, 3)];
        let c = constats(&v, 3);
        assert!(c.iter().any(|l| l.contains("plus de 5 h de jeu : 0 séance")), "{c:?}");
        assert_eq!(un(2.25), "2,3");
    }
}
