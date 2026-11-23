//! Sources de données, toutes en lecture seule et facultatives (règle 4 de DIRECTION.md) :
//! une source absente, illisible ou trop lente renvoie `None`, sans erreur affichée.

pub mod git;
pub mod jeu;
pub mod memoire;
pub mod sport;

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Commande sans console visible (règle §0 : rien ne doit apparaître à l'écran).
pub fn commande(exe: &str) -> Command {
    let mut c = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    c
}

/// Lance `c` et renvoie sa sortie standard si elle se termine avec succès avant `delai`.
/// Au-delà, le processus (le nôtre, on a son handle) est tué.
pub fn sortie(mut c: Command, delai: Duration) -> Option<Vec<u8>> {
    let mut enfant = c.spawn().ok()?;
    let mut out = enfant.stdout.take()?;
    // Lecture dans un thread : un tube plein bloquerait l'enfant avant qu'il se termine.
    let lecteur = std::thread::spawn(move || {
        let mut v = Vec::new();
        std::io::Read::read_to_end(&mut out, &mut v).ok().map(|_| v)
    });
    let fin = Instant::now() + delai;
    loop {
        match enfant.try_wait() {
            Ok(Some(st)) => return if st.success() { lecteur.join().ok().flatten() } else { None },
            Ok(None) if Instant::now() < fin => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = enfant.kill();
                let _ = enfant.wait();
                return None;
            }
        }
    }
}
