//! Tâche planifiée Windows « Bilan » : chaque jour à 10 h (ou dès que le PC est allumé),
//! lance `Bilan.exe --auto`, qui photographie le temps de jeu et n'ouvre la fenêtre
//! que si le bilan de la semaine écoulée n'a pas encore été vu. Pas d'admin : déclencheur horaire.
//! Activée seulement depuis les réglages, jamais pendant les tests (règle §0).

use crate::sources::{commande, sortie};
use std::time::Duration;

const NOM: &str = "Bilan (ouverture du lundi)";

fn xml(exe: &str) -> String {
    let exe = exe.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers><CalendarTrigger><StartBoundary>2026-01-05T10:00:00</StartBoundary><ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay></CalendarTrigger></Triggers>
  <Principals><Principal id="Author"><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
  <Settings><StartWhenAvailable>true</StartWhenAvailable><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><ExecutionTimeLimit>PT0S</ExecutionTimeLimit><Priority>7</Priority></Settings>
  <Actions Context="Author"><Exec><Command>{exe}</Command><Arguments>--auto</Arguments></Exec></Actions>
</Task>"#
    )
}

/// Crée (ou remplace) la tâche pour l'exe courant.
pub fn activer() -> Result<(), String> {
    if socle::testmode::enabled() {
        return Err("mode test : tâche non créée".into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let f = std::env::temp_dir().join("bilan-tache.xml");
    // schtasks attend de l'UTF-16 avec BOM quand l'en-tête l'annonce.
    let mut b = vec![0xFF, 0xFE];
    for u in xml(&exe.to_string_lossy()).encode_utf16() {
        b.extend_from_slice(&u.to_le_bytes());
    }
    std::fs::write(&f, b).map_err(|e| e.to_string())?;
    let mut c = commande("schtasks");
    c.args(["/Create", "/F", "/TN", NOM, "/XML"]).arg(&f);
    let ok = sortie(c, Duration::from_secs(10)).is_some();
    let _ = std::fs::remove_file(&f);
    if ok { Ok(()) } else { Err("schtasks a refusé la création de la tâche".into()) }
}

pub fn desactiver() -> Result<(), String> {
    let mut c = commande("schtasks");
    c.args(["/Delete", "/F", "/TN", NOM]);
    sortie(c, Duration::from_secs(10)).map(|_| ()).ok_or_else(|| "tâche introuvable ou suppression refusée".into())
}
