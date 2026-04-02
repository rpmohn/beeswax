use std::io::{self, Write};
use std::path::Path;

use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use argon2::Argon2;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::model::{Category, Item, View};

// ── File format constants ──────────────────────────────────────────────────────

const MAGIC: &[u8; 4]   = b"BWX\0";
const FLAG_PLAIN:     u8 = 0x00;
const FLAG_ENCRYPTED: u8 = 0x01;

pub const SCHEMA_VERSION: u32 = 3;

// ── SaveData ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub version:      u32,
    pub categories:   Vec<Category>,
    pub items:        Vec<Item>,   // global item pool shared across all views
    pub views:        Vec<View>,   // views[current_view] is the active view
    pub current_view: usize,
    pub next_id:      usize,
}

// ── Migration ─────────────────────────────────────────────────────────────────

/// v1 on-disk View — had items embedded.
#[derive(serde::Deserialize)]
struct ViewV1 {
    id:         usize,
    name:       String,
    sections:   Vec<crate::model::Section>,
    items:      Vec<Item>,
    columns:    Vec<crate::model::Column>,
    #[serde(default)]
    left_count: usize,
}

#[derive(serde::Deserialize)]
struct SaveDataV1 {
    #[allow(dead_code)]
    version:    u32,
    categories: Vec<Category>,
    view:       ViewV1,
    next_id:    usize,
}

/// v2 on-disk View — also had items embedded.
#[derive(serde::Deserialize)]
struct SaveDataV2 {
    #[allow(dead_code)]
    version:      u32,
    categories:   Vec<Category>,
    views:        Vec<ViewV1>,
    current_view: usize,
    next_id:      usize,
}

fn view_v1_to_view(v: ViewV1) -> View {
    View { id: v.id, name: v.name, sections: v.sections, columns: v.columns, left_count: v.left_count }
}

fn migrate(version: u32, json: &str) -> Result<SaveData, LoadError> {
    match version {
        1 => {
            let v1: SaveDataV1 = serde_json::from_str(json).map_err(|_| LoadError::Corrupt)?;
            let items = v1.view.items.clone();
            Ok(SaveData {
                version:      3,
                categories:   v1.categories,
                items,
                views:        vec![view_v1_to_view(v1.view)],
                current_view: 0,
                next_id:      v1.next_id,
            })
        }
        2 => {
            let v2: SaveDataV2 = serde_json::from_str(json).map_err(|_| LoadError::Corrupt)?;
            // Merge items from all views into the global pool (deduplicate by id).
            let mut seen_ids = std::collections::HashSet::new();
            let mut items: Vec<Item> = Vec::new();
            for view in &v2.views {
                for item in &view.items {
                    if seen_ids.insert(item.id) {
                        items.push(item.clone());
                    }
                }
            }
            let views: Vec<View> = v2.views.into_iter().map(view_v1_to_view).collect();
            Ok(SaveData {
                version:      3,
                categories:   v2.categories,
                items,
                views,
                current_view: v2.current_view,
                next_id:      v2.next_id,
            })
        }
        3 => serde_json::from_str(json).map_err(|_| LoadError::Corrupt),
        v => Err(LoadError::UnknownVersion(v)),
    }
}

// ── Save ──────────────────────────────────────────────────────────────────────

fn clone_view(view: &View) -> View {
    View {
        id:         view.id,
        name:       view.name.clone(),
        sections:   view.sections.iter().map(|s| crate::model::Section {
            id:               s.id,
            name:             s.name.clone(),
            cat_id:           s.cat_id,
            sort_new:         s.sort_new,
            primary_on:       s.primary_on,   primary_order:    s.primary_order,
            primary_cat_id:   s.primary_cat_id, primary_seq:   s.primary_seq,
            secondary_on:     s.secondary_on, secondary_order:  s.secondary_order,
            secondary_cat_id: s.secondary_cat_id, secondary_seq: s.secondary_seq,
        }).collect(),
        columns:    view.columns.iter().map(|c| crate::model::Column {
            id:       c.id,
            name:     c.name.clone(),
            cat_id:   c.cat_id,
            width:    c.width,
            format:   c.format,
            date_fmt: c.date_fmt.clone(),
        }).collect(),
        left_count: view.left_count,
    }
}

pub fn save_plain(
    path:           &Path,
    categories:     &[Category],
    items:          &[Item],
    view:           &View,
    inactive_views: &[View],
    next_id:        usize,
) -> io::Result<()> {
    let views: Vec<View> = std::iter::once(view)
        .chain(inactive_views.iter())
        .map(clone_view)
        .collect();
    let data = SaveData {
        version:      SCHEMA_VERSION,
        categories:   categories.to_vec(),
        items:        items.to_vec(),
        views,
        current_view: 0,
        next_id,
    };
    let json = serde_json::to_string(&data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut f = std::fs::File::create(path)?;
    f.write_all(MAGIC)?;
    f.write_all(&SCHEMA_VERSION.to_le_bytes())?;
    f.write_all(&[FLAG_PLAIN])?;
    f.write_all(json.as_bytes())?;
    Ok(())
}

pub fn save_encrypted(
    path:           &Path,
    password:       &str,
    categories:     &[Category],
    items:          &[Item],
    view:           &View,
    inactive_views: &[View],
    next_id:        usize,
) -> io::Result<()> {
    let views: Vec<View> = std::iter::once(view)
        .chain(inactive_views.iter())
        .map(clone_view)
        .collect();
    let data = SaveData {
        version:      SCHEMA_VERSION,
        categories:   categories.to_vec(),
        items:        items.to_vec(),
        views,
        current_view: 0,
        next_id,
    };
    let json = serde_json::to_string(&data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Generate random salt and nonce
    let mut salt  = [0u8; 32];
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce);

    // Derive key with Argon2id
    let key = derive_key(password, &salt)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Encrypt with AES-256-GCM
    let cipher     = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce_obj  = Nonce::from_slice(&nonce);
    let ciphertext = cipher.encrypt(nonce_obj, json.as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "encryption failed"))?;

    let mut f = std::fs::File::create(path)?;
    f.write_all(MAGIC)?;
    f.write_all(&SCHEMA_VERSION.to_le_bytes())?;
    f.write_all(&[FLAG_ENCRYPTED])?;
    f.write_all(&salt)?;
    f.write_all(&nonce)?;
    f.write_all(&ciphertext)?;
    Ok(())
}

// ── Load ──────────────────────────────────────────────────────────────────────

pub enum LoadResult {
    Plain(SaveData),
    NeedsPassword,
}

pub fn probe(path: &Path) -> io::Result<LoadResult> {
    let bytes = std::fs::read(path)?;
    let (flag, version, rest) = parse_header(&bytes)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "not a .bwx file"))?;
    match flag {
        FLAG_PLAIN => {
            let json = std::str::from_utf8(rest)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid UTF-8"))?;
            let data = migrate(version, json)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e:?}")))?;
            Ok(LoadResult::Plain(data))
        }
        FLAG_ENCRYPTED => Ok(LoadResult::NeedsPassword),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, "unknown format flag")),
    }
}

pub fn load_plain(path: &Path) -> io::Result<SaveData> {
    match probe(path)? {
        LoadResult::Plain(d) => Ok(d),
        LoadResult::NeedsPassword => Err(io::Error::new(io::ErrorKind::InvalidData, "file is encrypted")),
    }
}

pub fn load_encrypted(path: &Path, password: &str) -> Result<SaveData, LoadError> {
    let bytes = std::fs::read(path).map_err(LoadError::Io)?;
    let (flag, version, rest) = parse_header(&bytes).ok_or(LoadError::Corrupt)?;
    if flag != FLAG_ENCRYPTED { return Err(LoadError::Corrupt); }
    if rest.len() < 44 { return Err(LoadError::Corrupt); }  // 32 salt + 12 nonce

    let (salt,  rest) = rest.split_at(32);
    let (nonce, ciphertext) = rest.split_at(12);

    let key = derive_key(password, salt).map_err(|_| LoadError::Corrupt)?;
    let cipher    = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce_obj = Nonce::from_slice(nonce);
    let plaintext = cipher.decrypt(nonce_obj, ciphertext).map_err(|_| LoadError::WrongPassword)?;

    let json = std::str::from_utf8(&plaintext).map_err(|_| LoadError::Corrupt)?;
    migrate(version, json)
}

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    WrongPassword,
    Corrupt,
    UnknownVersion(u32),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(e)             => write!(f, "I/O error: {e}"),
            LoadError::WrongPassword     => write!(f, "Wrong password"),
            LoadError::Corrupt           => write!(f, "File is corrupt or unreadable"),
            LoadError::UnknownVersion(v) => write!(f, "Unknown file version {v}"),
        }
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Returns (flag, schema_version, payload_slice) or None if the header is invalid.
fn parse_header(bytes: &[u8]) -> Option<(u8, u32, &[u8])> {
    if bytes.len() < 9 { return None; }
    if &bytes[..4] != MAGIC { return None; }
    let version = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
    let flag    = bytes[8];
    Some((flag, version, &bytes[9..]))
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| e.to_string())?;
    Ok(key)
}
