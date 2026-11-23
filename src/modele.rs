//! Données d'une semaine, communes aux sources, aux constats et à l'affichage.
//! Une semaine va du lundi 00:00 au lundi suivant 00:00, en heure locale.

use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};

/// Lundi de la semaine qui contient `d`.
pub fn lundi_de(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

/// Bornes Unix (secondes) de la semaine : [début, fin[.
pub fn bornes(lundi: NaiveDate) -> (i64, i64) {
    (minuit(lundi), minuit(lundi + Duration::days(7)))
}

/// Minuit local du jour `d`, en secondes Unix (le plus tôt en cas d'heure ambiguë).
pub fn minuit(d: NaiveDate) -> i64 {
    let n = d.and_hms_opt(0, 0, 0).unwrap_or_default();
    Local.from_local_datetime(&n).earliest().map_or(n.and_utc().timestamp(), |t| t.timestamp())
}

/// Jour local d'un instant Unix.
pub fn jour_local(t: i64) -> NaiveDate {
    Local.timestamp_opt(t, 0).single().map_or(NaiveDate::default(), |d| d.date_naive())
}

/// Index du jour dans la semaine (0 = lundi), ou `None` hors de la semaine.
pub fn index_jour(lundi: NaiveDate, d: NaiveDate) -> Option<usize> {
    let i = (d - lundi).num_days();
    (0..7).contains(&i).then_some(i as usize)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sport {
    /// Jours où au moins une série a été notée.
    pub seances: u32,
    /// Séries par jour (lundi → dimanche).
    pub par_jour: [u32; 7],
    /// (nom du muscle, séries de la semaine, objectif), dans l'ordre du catalogue, muscles à 0 compris.
    pub muscles: Vec<(String, u32, u32)>,
    /// Somme charge × répétitions (kg) des séries de musculation.
    pub volume_kg: f64,
    /// Exercices dont la charge maximale dépasse celle des 4 semaines précédentes : (nom, avant, maintenant).
    pub progres: Vec<(String, f64, f64)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Jeu {
    /// Minutes estimées pour la semaine.
    pub minutes: u32,
    /// Jeux les plus joués : (nom, minutes), décroissant.
    pub top: Vec<(String, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dev {
    pub commits: u32,
    pub par_jour: [u32; 7],
    /// (nom du dépôt, commits), décroissant.
    pub projets: Vec<(String, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Memoire {
    pub copies: u32,
    pub captures: u32,
    pub par_jour: [u32; 7],
}

/// Tout ce que l'on sait d'une semaine. `None` = source absente ou sans donnée pour cette semaine.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Semaine {
    pub lundi: NaiveDate,
    pub sport: Option<Sport>,
    pub jeu: Option<Jeu>,
    pub dev: Option<Dev>,
    pub memoire: Option<Memoire>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lundi_et_bornes() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap(); // dimanche
        let l = lundi_de(d);
        assert_eq!(l, NaiveDate::from_ymd_opt(2026, 11, 16).unwrap());
        let (a, b) = bornes(l);
        assert_eq!(b - a, 7 * 86400);
        assert_eq!(jour_local(a), l);
        assert_eq!(index_jour(l, d), Some(6));
        assert_eq!(index_jour(l, l + Duration::days(7)), None);
    }
}
