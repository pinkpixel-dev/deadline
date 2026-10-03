# Changelog

## 0.2.1 - October 3, 2026

### 🐛 Fixes

- A wrong archive key now reminds you of the key you already learned, or points you at who knows it
- ghost_17 nudges you if you know the key but haven't opened board 08, and again if you're in the archive but haven't read 804
- If ghost_17 is too angry to help, the system log gives the hint instead


## 0.2.0 - October 3, 2026

### 🎮 Act II: ARCHIVES

- Act II starts on your first login after Act I ends, with a title card and a different opening for each of Act I's four endings
- Board 08 grows by ten archive logs, and `/archive` opens with the consent log, a meetup photo and more
- Eli is online all night and slowly works out that it isn't 1998
- null leaks JANUS source and asks for three favors: recover a seal, unseal node 07, shadow Eli
- Parallax calls from outside for the consent log, with a new `send <user> <file>` command
- ROOT asks you to keep Eli in the dark, and lowers your access level if you push too hard
- `[10] YOU` and `[11] DON'T GO IN HERE` appear and disappear
- Four Act II finales, one per route, ending on the 1998 cliffhanger
- New `shadow <user>` command and Act II versions of `listen` and `trace`

### 🌙 Act breaks

- Once an act is finished you get about five minutes of free roam, then ROOT warns you and hangs up
- The next act begins on your next login

### 🐛 Fixes

- Story events now belong to an act, so Act I scenes can't fire during Act II


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

### 🖱️ Navigation

- Board, post, mail, file, user and help lists are clickable, with hover and keyboard selection (↓ on an empty prompt)
- Action rows under posts, mail and files: next, back, download, inspect, recover
- New mail and private message notices can be tapped
- Bare numbers and names from the current list work as input (`03`, `102`, `nodelist.txt`)
- `read`, `view` and friends show the relevant list when you leave off the argument, and boards open by name or prefix

### 🛠️ Engine

- Data-driven story in RON files, with conditions, effects, events and scripted command hooks
- `--check` validates story content, `--reset` starts a fresh session
- Tab completion, history browsing, scrollback and mouse support
