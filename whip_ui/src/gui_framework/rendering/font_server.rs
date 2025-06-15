use bevy_ecs::system::Resource;
use bevy_log::warn;
use cosmic_text::{fontdb, FontSystem};
use std::sync::{Arc, Mutex};

// Manages the font loading and access using cosmic-text and fontdb
#[derive(Debug)]
pub struct FontServer {
    // FontSystem provides shaping, layout, and font fallback
    pub font_system: FontSystem,
    // Database holds loaded font data
    pub font_database: fontdb::Database,
}

impl FontServer {
    pub fn new() -> Self {
        // --- Load Fonts using fontdb ---
        let mut db = fontdb::Database::new();
        // Load system fonts. This can take a moment.
        // Consider loading specific fonts for faster startup or embedding fonts.
        db.load_system_fonts();
        let face_count = db.faces().count();

        if face_count == 0 {
            warn!("[FontServer::new] No system fonts found or loaded! Text rendering might fail.");
            // Consider adding a fallback mechanism or erroring here depending on requirements.
            // For now, we'll proceed with an empty FontSystem if no fonts are found.
        }

        // --- Create FontSystem ---
        // Pass the fontdb Database to FontSystem.
        // FontSystem uses this database to find appropriate fonts for characters.
        let font_system = FontSystem::new_with_locale_and_db("en-US".into(), db.clone());

        Self {
            font_system,
            font_database: db, // Store the database if needed elsewhere, otherwise FontSystem holds refs
        }
    }

    // Add methods later to query fonts, get font IDs, etc. if needed directly
    // pub fn get_font_id(&self, /* query criteria */) -> Option<fontdb::ID> { ... }
}

// --- Bevy Resource ---

// Using Arc<Mutex> for interior mutability, although FontSystem itself might be Send+Sync
// depending on cosmic-text version. Mutex provides safety regardless.
#[derive(Resource, Clone)]
pub struct FontServerResource(pub Arc<Mutex<FontServer>>);