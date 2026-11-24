# Bilan — stack technique (document de référence)

Version 0.1.0, v1 livrée vite (consigne du 24/11) : pas encore d'audit ponytail ni d'EGM.

## 1. But et direction
- **Rôle** : chaque semaine (lundi → dimanche, heure locale), une page qui croise sport, jeu, dev et vidéos, avec des constats factuels.
- **Principes** : lecture seule de toutes les sources, toutes facultatives (règle 4) ; rien ne sort du PC ; seulement des compteurs, des noms de jeux, de dépôts et de chaînes (jamais de titre de vidéo ni de contenu de Mémoire).
- **Hors périmètre v1** : bloc vidéos (Tube n'a pas d'historique local daté), sessions de jeu exactes (Ultimate ne les garde pas).

## 2. Stack
| Domaine | Choix |
|---|---|
| Langage | Rust 2024, rustc 1.99.1 MSVC |
| UI | egui 0.37 + winit 0.30 + rendu CPU (`hdrshot-egui-backend` + `softbuffer`), boucle copiée de l'explorateur |
| Socle | v0.1.6, features `ui settings crash instance update testmode` |
| Dates | `chrono` 0.4 (heure locale) |
| Outils externes | `git` (dev), `schtasks` (tâche du lundi), `Memoire.exe` (facultatif) |

## 3. Architecture
| Fichier | Rôle |
|---|---|
| `src/main.rs` | boucle winit, rendu, `--auto`, capture de test (`BILAN_CAPTURE`) |
| `src/app.rs` | réglages, chargement en thread, page (cartes, barres, jauges), Ctrl+K, F1, mise à jour |
| `src/modele.rs` | semaine (bornes locales), structures `Sport`, `Jeu`, `Dev`, `Memoire`, `Semaine` |
| `src/constats.rs` | phrases : tendance sur 4 semaines, croisement jeu × sport et jeu × dev par médiane |
| `src/tache.rs` | tâche planifiée « Bilan (ouverture du lundi) » par XML, sans admin |
| `src/sources/git.rs` | dépôts (dossier `.git`, 2 niveaux, worktrees ignorés, préfixes exclus : `godot`), `git log --all --no-merges --author --since` en parallèle |
| `src/sources/jeu.rs` | photos `history.json` d'Ultimate + photos de `library.json` prises par Bilan (`%APPDATA%\Bilan\jeu-photos.json`) ; minutes = écart entre la dernière photo avant le lundi et la dernière du dimanche |
| `src/sources/sport.rs` | export `appli-sport-*.json` le plus récent ; catalogue recopié de l'appli ; séances = jours distincts ; séries par muscle (unilatéral = max G/D) vs objectif |
| `src/sources/memoire.rs` | `Memoire.exe --stats-semaine --json --semaine AAAA-MM-JJ`, 3 s max |

## 4. Direction artistique
Thème du socle (crème / bleu nuit), cartes arrondi 12, en-têtes 11 px `muted`, icônes vectorielles. Couleurs : sport `add`, dev `accent`, Mémoire `live`, jauge sous l'objectif `live`.

## 5. Conventions
Réglages `%APPDATA%\Bilan\settings.json` (`auteur`, `racines`, `exclure`, `memoire_exe`, `auto`, `dernier_vu`). Ctrl+K, F1, ←/→, F5. Processus externes sans console (`CREATE_NO_WINDOW`) et avec délai.

## 6. Fonctionnalités
- 12 semaines chargées en un passage (threads), semaine terminée affichée par défaut, semaine en cours marquée « provisoire ».
- `--auto` : photo du temps de jeu, puis fenêtre ouverte sans focus seulement si le bilan de la semaine écoulée n'a pas été vu.
- Tâche planifiée quotidienne à 10 h (`StartWhenAvailable`), activable dans le menu ⋯, désactivée par défaut.
- Mise à jour `socle::update` (Desarrh/bilan).

## 7. Mesures (24/11, mode test invisible)
| Mesure | Valeur |
|---|---|
| 1re image | 32 ms |
| Chargement des sources (12 semaines, 9 dépôts) | ≈ 260 ms |
| Exe release | 5,6 Mo |
| RAM, CPU au repos | non mesurés |

## 8. Limites connues
- Jeu **approximatif** : Ultimate ne photographie qu'au rafraîchissement de sa bibliothèque (jour UTC) ; Bilan ajoute une photo à chaque lancement. Un écart entre deux photos éloignées est attribué à la semaine de la photo finale.
- Sport : dépend d'un export manuel ; une semaine après la date du fichier reste vide.
- Tâche planifiée jamais créée pendant les tests (§0) : non vérifiée sur la machine.
- Pas de tests d'interface ; `cargo test` : 10 tests (sources, constats, semaine).

## 9. Pistes (à valider)
- Ultimate : `sessions.jsonl` (sessions datées) pour un temps de jeu exact.
- Tube : `vues.jsonl` pour le bloc vidéos.
- Comparaison avec la même semaine du mois précédent, export HTML de la page.
