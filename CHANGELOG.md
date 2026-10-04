# Changelog

## 0.10.0 - October 3, 2026

### 📺 Title screen

- The game opens on a short animated title now. A CRT warms up, the modem handshake plays as carrier waves on a scope, the waves lock into one line, and that line breaks apart into a big DEADLINE. It takes about five seconds, and any key or click skips it

### 🎨 Art

- Every act's title card has its own animated piece: the modem for Act I, turning tape reels for Act II, a 21:04 clock that sometimes reads 03:17 for Act III, you rendering a scanline at a time for Act IV, and a half-violet face for Act V
- The cutaway scenes animate too. Rebuilds and the `before_you` replay get a clock running backwards, BURN and PURGE watch the nodes go dark, ESCAPE and ARCHIVE watch them light up, HUMAN shows the trace running back to your machine, and ROOT's new caller gets a modem
- JANUS shows its face the first time it talks to you
- The credits open with the modem's lights going out one at a time
- Boards GENERAL, CODE, OFF-TOPIC, SYSOP, ARCHIVE and YOU have header art now

### 🔁 Later runs

- Act III's 1998 builds start three higher for every run you've finished. On a second run the night opens at BUILD 1998.4, and everything that mentions a build number (the header, `date`, the journal, the build log, ghost's line, Eli's finger and Act IV's opening) agrees with it

### 🧪 Tests

- Every ending is now played from a saved end-of-Act-IV session, launched the way the game launches, through the ending and credits to the hang-up, and then launched again to check the next run starts fresh. Each one is also checked one step short of its gate

### 🏷️ Versioning

- Bumped to 0.10.0

## 0.9.0 - October 3, 2026

### 🗝️ Act V: the core

- JANUS leaves `/var/janus/core` open on the last night. There are four files in it, and each one opens a different way: Mara's key, `recover`, tracing yourself, and `listen`
- The files dig into how JANUS survived, what the test is, why node 02 reads exactly like a lurker from 1998, and what really happened at 03:17. None of them settle it. They're meant to be read more than one way
- Each file ends with a key fragment. Put the four together and a fifth file opens, `janus.self`, and then JANUS pages you for one last conversation where you can ask it anything
- Something on node 03 reads along behind you. `scan` and `trace 03` will tell you a little about it
- `whoami`, `scan` and your command history get a bit stranger the deeper you go, and JANUS writes in your notes again
- Everyone can use `recover` and `listen` on the last night, even if you never learned them earlier
- None of this is required. Every ending command still works from the first minute, and reading the core only nudges a couple of gates by a point
- Every ending picks up a line or two about what you found in the core, like drive 2 finally powering off, node 03 logging off, or the test spec loading in JANUS v2.0

### 🐛 Fixes

- Act III's `BUILD 1998.2` and `1998.3` label now stays in the header for the whole rebuilt night. It used to vanish after five seconds, so the journal mentioned something most people never saw
- Once the night rebuilds, `date` shows which build it is, ghost_17 asks why your login says BUILD, and the journal points you at `scan`

### ⏱️ Timing

- ROOT now waits 60 minutes before hanging up on the last night (it was 40), so there's time to actually read the core

## 0.8.0 - October 3, 2026

### 🎬 Act V: last talks

- Before you decide, everyone gets one last conversation: ghost_17, Parallax, null, ROOT, and Eli if he knows what he is
- Each talk can nudge you over the line for one ending. Parallax can hand over the kill code, ghost can come to terms with their file, null asks one last time, ROOT tells you where the sysop commands are, and Eli can still learn the truth about the article you deleted
- None of them are required. Every ending command still works from the first minute if you've earned it

### 🎞️ Endings

- Every ending scene now reflects your run: who you were close to, what you told people, your promise to ROOT on the first night, and how the last talks went

### 🔁 Replay

- On a later run, the characters notice. ghost says it feels like a rerun, Parallax thinks they've had this call before, and Eli asks if you've already said goodbye
- Two posts on Act II's `[10] YOU` board read differently once you've seen an ending

## 0.7.0 - October 3, 2026

### 🎬 Act V: DECISION

- The last night is playable. JANUS opens the act and frames it by whichever way you leaned all game: keeping things, burning them, or letting them out
- `options` lists everything that can be done with JANUS. Each ending is a typed command (`burn`, `burn --keep-source`, `preserve`, `fork`, `prove`, `stay`, `override`, `isolate eli`), and they all work from the first minute if you've earned them
- Locked endings still answer, and say who's in the way: Parallax won't give you the kill code, null isn't answering, ROOT refuses
- Every ending asks you to confirm first, since it ends the run
- Nine endings, including a secret one, each with its own closing scene and a credits screen that tracks which ones you've found
- If you never decide, ROOT eventually hangs up, and JANUS picks it back up on your next login

### 🔁 Replay

- Finishing an ending ends the run. Your next launch starts a fresh game, and everything the game remembers across runs stays
- The opening screen picks up a small detail for every ending you've seen

### 🧹 Maintenance

- New story effects `Ending(id)` and `Rebuild`, and conditions `More(a, b)` and `Endings(n)`
- Nothing fires after the line has hung up

## 0.6.3 - October 3, 2026

### 🐛 Fixes

- Logging in after the last finished act used to drop you on a quiet board with nothing left to happen. The game now says you've reached the end of the current build, and your save picks up from there once the next act is written

## 0.6.2 - October 3, 2026

### 🐛 Fixes

- The session now autosaves every 10 commands and whenever an act ends, you restore a snapshot or the story rewinds you. Before, it only saved on `logout`, Ctrl+C or ROOT's kick, so closing the terminal window could throw away a whole act
- Snapshots JANUS planted (like `before_you`) are cleared when a new act starts, so a leftover from an earlier run can't be restored before Act IV writes it again
- Restoring `before_you` while you were already inside it made the replay its own way back, so it looped forever. It now keeps the original return point and always drops you back in your own timeline

## 0.6.1 - October 3, 2026

### 🐛 Fixes

- ROOT's second Act II lockout (at `root_alert` 12) never lifted, so `download`, `recover`, `chat`, `send` and the file commands stayed blocked for the rest of the act. It now lifts after 10 commands, and existing saves stuck in it recover the same way
- The blocked-command message now adds `ROOT is watching. try again later.` so the lockout reads as temporary instead of broken

## 0.6.0 - October 3, 2026

### 🪞 Act IV: IDENTITY

- Act IV opens on the next login after Act III. `/var/janus/models` now has five models in it: `mara.q`, `eli.v`, `ghost17`, `parallax.partial` and `player_04`, which has your handle on it
- `inspect player_04` shows the memory seed, the confidence and the session it's connected to. `whoami` and `trace me` change once you've seen it
- ghost_17 asks you to look in the folder for them. You can tell them about `ghost17` or hide it, and how they take it depends on how much they trust you. Hiding it doesn't stick
- Parallax, ROOT, null and ghost each have a different theory about what you are. ROOT remembers your Act I promise, and might slip if it trusts you
- Parallax wants proof you're outside. `date --local` shows your machine's real clock next to the board's
- `trace null` finally resolves all the way, and null has to answer for it
- JANUS writes a snapshot you never made, `before_you`, and leaves a note in your journal. Restoring it plays the first night with your choice about ghost_17 flipped, then sends you back
- If Eli knows what happened to him (or found the deletion log), he comes back on node 04 in the present
- The act ends with JANUS asking what you are. You answer with `reply janus`, then the end card plays and ROOT hangs up five minutes later

### 🧹 Maintenance

- New `PlantSnapshot` effect writes a snapshot from the current timeline with some state changed. Restoring one remembers where you were, so the story can `Rewind` you back
- New `Note` effect writes into the player's own journal notes
- New `{now}` markup for the player's real local time
- Eli's age in post 503 now matches his portrait (20)

### 🐛 Fixes

- Act II's system log lines (like `node 04: indexing 06/1998` and `ROOT: archive mount verified`) no longer show up in Act III and Act IV. Ambient lines now only appear during their own act, the same way events work

## 0.5.0 - October 3, 2026

### 📼 Act III: 1998 (complete)

- Changing the night now rebuilds it: back to 9:04 PM as `BUILD 1998.2`, then `1998.3`. What you tried carries over, and some people half remember it
- Each build plays a little differently. Eli, Mara and ghost_17 have new lines, the bulletin changes, `scan` shows node 04 as JANUS, and a diner photo shows up before the diner happens
- Letting Eli run the archiver, or changing anything in the third build, skips to dawn on 08/15/1998 and the newspaper appears
- Eli asks if you know what happened to him. You answer with commands: `reply eli`, `send eli eli_voss.txt`, `delete` the article, or try to `logout`
- Present-day parallax calls in on the external line, then the night collapses and you're back in the present with a new `/var/janus` directory
- Act III ends with its own end card, and ROOT hangs up five minutes later like the other acts

### 🧹 Maintenance

- New `Mark` and `Rewind` story effects for returning to an earlier moment while keeping chosen flags, vars, history, notes and the journal

## 0.4.0 - October 3, 2026

### 📼 Act III: 1998 (first half)

- Act III opens on the next login after Act II: the night of 08/14/1998, starting at 9:04 PM, with all nine users alive and online
- Everyone has something to say: eli, mara, a younger ghost_17, a 1998 parallax, kestrel, dialtone, byte_witch and crankshaft
- The board is live. New posts show up while you're online, and the pizza thread grows as the night goes on
- Eli's going-away meetup happens at the diner. Everyone logs off, and the board does something strange with the time while they're gone
- Four ways to try to change the night: warn eli, tell mara, keep eli talking until he goes to bed, or use his back door into node 04
- Present-day content (the archive board, `/archive`, replies dated today) is hidden during 1998
- The night stops at its first rebuild for now. The rest of Act III comes next

### ⌨️ Commands

- `reply <user>` answers or opens a channel with someone by name
- `send <user> <file>` works for any file in local storage, not just the one parallax asks for

### 🧹 Maintenance

- Story hooks from the act you're playing now run before older acts' hooks
- New `Clock` story effect sets the BBS time, and the header can show a `BUILD 1998.n` label

## 0.3.0 - October 3, 2026

### 📓 Journal

- New `journal` command (also `notes`): a notes file on your side of the modem that keeps what you've learned between sessions
- Leads show what's still open and get crossed off when you resolve them, so you can pick up where you left off days later
- Keys and codes, people, downloaded files and discovered commands are collected automatically
- `note <text>` adds your own notes, and `note rm <#>` removes one. Your notes survive `restore`
- Journal rows are tappable, and a single `*** JOURNAL UPDATED` notice appears when something new is added
- Existing saves get their journal filled in from what you've already discovered


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
