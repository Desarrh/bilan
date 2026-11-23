# Reprise de Bilan (arrêt du 23/11 au soir)

**Fait.**
- Sources lues. Le détail est dans les notes partagées `bilan.md`.
- Squelette posé : `Cargo.toml` (egui 0.37, rendu CPU, socle v0.1.6, chrono), `build.rs` et `src/modele.rs` (semaine, structures Sport/Jeu/Dev/Memoire, test).
- `src/sources/mod.rs` : commande sans console et délai d'attente.
- Fichiers vides pour git, jeu, sport et memoire. `main.rs` est vide et rien n'a encore été compilé.

**Réponses de l'utilisateur.**
- 4 blocs (sport, jeu, dev, vidéos), plus 2 ou 3 constats croisés factuels.
- Semaine du lundi au dimanche, bilan prêt le lundi.
- Une fenêtre, et une tâche planifiée qui l'ouvre le lundi. Elle s'active dans les réglages et reste désactivée par défaut.
- Sport : l'utilisateur exporte à la main chaque semaine. Bilan lit le `appli-sport-*.json` le plus récent, dans Téléchargements ou `%APPDATA%\Bilan\sport`.
- Dev : commits de « Desarrh » dans C:\Dev, sauf godot. Seulement des compteurs, des noms de jeux et des noms de chaînes.
- Jeu : heures approximatives, tirées de `history.json` d'Ultimate, à signaler dans l'interface.
- Vidéos : bloc masqué tant que Tube n'a pas de source. Les ajouts dans Ultimate et Tube viendront plus tard.
- Mémoire fournira `--stats-semaine --json [--semaine AAAA-MM-JJ]` → `{"v":1,"debut","copies","captures","par_jour":[7]}`.
- Consigne : aller au plus vite. Une v1 qui compile et se lance, un seul `cargo test`, pas d'audit ni d'EGM pour l'instant. Ensuite publier, installer dans `dist\` et envoyer un rapport court.

**Reste.**
1. Sources :
   - git : `--all --no-merges --author --since`, et seulement les dossiers `.git` (pas les worktrees) ;
   - jeu : écarts entre photos de `history.json`, plus des photos de `library.json` faites par Bilan ;
   - sport : catalogue recopié ; séances = jours distincts ; séries par muscle avec unilatéral = max(G, D), comparées à l'objectif, défaut 4 ;
   - memoire : lancer l'exe avec un délai de 3 s.
2. `constats.rs` : tendance sur 4 semaines, croisement jeu et sport par médiane.
3. Fenêtre : `main.rs` copié de l'explorateur, `app.rs` et `ui.rs` (←/→ entre semaines, Ctrl+K, F1).
4. `--auto` : photo du jeu, et ouverture sans focus (`with_active(false)`) si le bilan n'a pas été vu. Tâche planifiée par XML (déclencheur quotidien, StartWhenAvailable, sans admin) à créer ou supprimer depuis les réglages. Ne jamais la créer pendant les tests (§0).
5. Icône, STACK.md, README, dépôt `Desarrh/bilan`, release, installation dans `dist\`.
