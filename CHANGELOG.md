# Changelog

## 0.1.0 - October 3, 2026

### 🎮 Game

- First playable build with Act I: CONNECTION
- The board logs you in as your OS username (`$USER`) before you type anything
- Four Act I endings, picked by who you trusted and how you got into the archive
- Hidden state for trust, honesty, privacy and curiosity that changes what characters will share
- ghost_17, ROOT, null and eli conversations with branching replies, plus a cast of old board regulars
- Nine message boards with 55 posts, a file area, mail and `finger` plans
- Files that change between viewings, with checksums that notice
- Planted command history, a board that disappears and a help entry that shouldn't be there
- `snapshot` and `restore`, with a persistent memory that survives restores

### 🎨 Visuals

- Half-block pixel art portraits, an animated title scene and a 5x7 pixel banner font
- Modem dial-in, logout and act-end cinematics
- ROOT overlays that sometimes ignore Esc, plus screen glitch effects
- Responsive layout: the sidebar shows on wide terminals and the UI stays usable on narrow ones

### 🛠️ Engine

- Data-driven story in RON files, with conditions, effects, events and scripted command hooks
- `--check` validates story content, `--reset` starts a fresh session
- Tab completion, history browsing, scrollback and mouse support
