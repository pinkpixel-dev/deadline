pub mod cond;
pub mod effect;
pub mod model;
pub mod validate;

use std::collections::HashMap;

use anyhow::{Context, Result};
use include_dir::{Dir as EmbeddedDir, include_dir};

use crate::ui::pixel::Sprite;
use model::*;

static STORY: EmbeddedDir = include_dir!("$CARGO_MANIFEST_DIR/assets/story");
static SPRITES: EmbeddedDir = include_dir!("$CARGO_MANIFEST_DIR/assets/sprites");
static ART: EmbeddedDir = include_dir!("$CARGO_MANIFEST_DIR/assets/art");

/// Every piece of story content, merged from all embedded `.ron` files.
#[derive(Default)]
pub struct Content {
    pub boards: Vec<Board>,
    pub posts: Vec<Post>,
    pub mail: HashMap<String, Mail>,
    pub files: Vec<FileEntry>,
    pub dirs: Vec<Dir>,
    pub users: Vec<User>,
    pub dialogues: HashMap<String, Dialogue>,
    pub events: Vec<Event>,
    pub hooks: Vec<Hook>,
    pub overlays: HashMap<String, Overlay>,
    pub sequences: HashMap<String, Sequence>,
    pub ambient: Vec<Ambient>,
    pub help: Vec<HelpEntry>,
    pub sprites: HashMap<String, Sprite>,
    pub art: HashMap<String, String>,
}

fn ron_options() -> ron::Options {
    use ron::extensions::Extensions;
    ron::Options::default()
        .with_default_extension(Extensions::IMPLICIT_SOME | Extensions::UNWRAP_VARIANT_NEWTYPES)
}

fn walk<'a>(dir: &'a EmbeddedDir<'a>, out: &mut Vec<&'a include_dir::File<'a>>) {
    out.extend(dir.files());
    for d in dir.dirs() {
        walk(d, out);
    }
}

impl Content {
    pub fn load() -> Result<Self> {
        let mut c = Content::default();
        let mut files = Vec::new();
        walk(&STORY, &mut files);
        files.sort_by_key(|f| f.path().to_path_buf());
        let opts = ron_options();
        for f in files {
            if f.path().extension().and_then(|e| e.to_str()) != Some("ron") {
                continue;
            }
            let text = f.contents_utf8().context("story file is not utf-8")?;
            let part: ContentFile = opts
                .from_str(text)
                .with_context(|| format!("parsing {}", f.path().display()))?;
            c.merge(part);
        }
        c.posts.sort_by_key(|p| p.id);

        let mut sprites = Vec::new();
        walk(&SPRITES, &mut sprites);
        for f in sprites {
            let name = stem(f.path());
            let text = f.contents_utf8().context("sprite is not utf-8")?;
            let sprite = Sprite::parse(text).with_context(|| format!("sprite {name}"))?;
            c.sprites.insert(name, sprite);
        }
        let mut art = Vec::new();
        walk(&ART, &mut art);
        for f in art {
            let text = f.contents_utf8().context("art is not utf-8")?;
            c.art.insert(stem(f.path()), text.trim_end().to_string());
        }
        Ok(c)
    }

    fn merge(&mut self, p: ContentFile) {
        self.boards.extend(p.boards);
        self.posts.extend(p.posts);
        self.files.extend(p.files);
        self.dirs.extend(p.dirs);
        self.users.extend(p.users);
        self.events.extend(p.events);
        self.hooks.extend(p.hooks);
        self.ambient.extend(p.ambient);
        self.help.extend(p.help);
        self.mail.extend(p.mail.into_iter().map(|m| (m.id.clone(), m)));
        self.dialogues
            .extend(p.dialogues.into_iter().map(|d| (d.id.clone(), d)));
        self.overlays
            .extend(p.overlays.into_iter().map(|o| (o.id.clone(), o)));
        self.sequences
            .extend(p.sequences.into_iter().map(|s| (s.id.clone(), s)));
    }

    /// Find a board by number (`01`, `1`), name (`general`) or a name
    /// prefix of at least three letters (`trading`, `off`).
    pub fn board(&self, id: &str) -> Option<&Board> {
        let num = id.trim_start_matches('0');
        let norm = |s: &str| -> String {
            s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase()
        };
        let want = norm(id);
        self.boards
            .iter()
            .find(|b| b.id.trim_start_matches('0') == num)
            .or_else(|| self.boards.iter().find(|b| !want.is_empty() && norm(&b.name) == want))
            .or_else(|| {
                (want.len() >= 3)
                    .then(|| self.boards.iter().find(|b| norm(&b.name).starts_with(&want)))
                    .flatten()
            })
    }

    pub fn post(&self, id: u32) -> Option<&Post> {
        self.posts.iter().find(|p| p.id == id)
    }

    pub fn user(&self, id: &str) -> Option<&User> {
        self.users.iter().find(|u| u.id.eq_ignore_ascii_case(id))
    }

    pub fn dir(&self, path: &str) -> Option<&Dir> {
        self.dirs.iter().find(|d| d.path == path)
    }
}

fn stem(p: &std::path::Path) -> String {
    p.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

/// Pick the first matching alt body, falling back to the base body.
pub fn pick<'a>(
    base: &'a str,
    alts: &'a [Alt],
    st: &crate::game::state::GameState,
    meta: &crate::game::state::MetaState,
) -> &'a str {
    alts.iter()
        .find(|a| a.cond.eval(st, meta))
        .map(|a| a.body.as_str())
        .unwrap_or(base)
}
