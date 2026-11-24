# Bilan

Weekly review for Windows that crosses sport, gaming, dev and (later) video data into one page, with plain factual findings
("3 sessions, 0 on weeks with more than 15 h of gaming"). Rust + egui, CPU rendering, single portable exe, everything stays local.

Sources (all optional and read-only, a missing source is simply skipped):
- **Dev**: `git log` of the repositories under `C:\Dev`, filtered by author.
- **Gaming**: playtime snapshots of the Ultimate launcher (`%APPDATA%\dev.local.launcher`), estimated per week.
- **Sport**: the latest `appli-sport-*.json` export (Downloads or `%APPDATA%\Bilan\sport`).
- **Clipboard & screenshots**: `Memoire.exe --stats-semaine --json`, when installed.

Keys: ←/→ previous/next week, F5 reload, Ctrl+K command palette, F1 help.
An optional scheduled task (menu ⋯, off by default) opens Bilan on Monday without stealing focus.

License: MIT.
