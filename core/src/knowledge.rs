//! Knowledge Base & Compatibility Engine.
//!
//! Repository over SQLite + JSON/YAML knowledge files for mappings, version/loader
//! compatibility, breaking changes, migration recipes, and search.
//!
//! Tasks: 5.1 SQLite schema + migrations, 5.2 Mapping import/export,
//! 5.3 Version/loader compatibility matrix, 5.4 Breaking change DB,
//! 5.5 Migration recipes (CRUD + versioning), 5.6 Knowledge search,
//! 5.7 Schema versioning + seed data.

use crate::persistence::DbError;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

/// API mapping between versions/loaders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mapping {
    pub version: String,
    pub loader: String,
    pub from_api: String,
    pub to_api: String,
    pub kind: String,
    pub source: String,
    pub confidence: f64,
}

/// Version/loader compatibility matrix entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Compatibility {
    pub minecraft_version: String,
    pub loader: String,
    pub loader_version: String,
    pub status: String,
    pub notes: Option<String>,
}

/// Known breaking change entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreakingChange {
    pub version: String,
    pub change: String,
    pub severity: String,
    pub workaround: Option<String>,
    pub source: String,
}

/// Reusable migration recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub recipe_id: String,
    pub version: String,
    pub loader: Option<String>,
    pub description: String,
    pub steps: Vec<Value>,
    pub source: String,
    pub tested: bool,
}

/// Mod dependency metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModDependency {
    pub mod_id: String,
    pub name: Option<String>,
    pub version: String,
    pub loader: Option<String>,
    pub url: Option<String>,
    pub checksum: Option<String>,
    pub source: String,
}

/// SQLite knowledge-base repository.
pub struct KnowledgeBase {
    conn: Connection,
}

impl KnowledgeBase {
    pub fn open(path: &Path) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    /// Schema versioning table + all data tables.
    const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER PRIMARY KEY,
        applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
        description TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS mappings (
        version TEXT NOT NULL,
        loader TEXT NOT NULL,
        from_api TEXT NOT NULL,
        to_api TEXT NOT NULL,
        kind TEXT NOT NULL,
        source TEXT NOT NULL,
        confidence REAL NOT NULL,
        PRIMARY KEY (version, loader, from_api, to_api, kind, source)
    );

    CREATE TABLE IF NOT EXISTS compatibility (
        minecraft_version TEXT NOT NULL,
        loader TEXT NOT NULL,
        loader_version TEXT NOT NULL,
        status TEXT NOT NULL,
        notes TEXT,
        PRIMARY KEY (minecraft_version, loader, loader_version)
    );

    CREATE TABLE IF NOT EXISTS breaking_changes (
        version TEXT NOT NULL,
        change TEXT NOT NULL,
        severity TEXT NOT NULL,
        workaround TEXT,
        source TEXT NOT NULL,
        PRIMARY KEY (version, change, severity, source)
    );

    CREATE TABLE IF NOT EXISTS recipes (
        recipe_id TEXT PRIMARY KEY,
        version TEXT NOT NULL,
        loader TEXT,
        description TEXT NOT NULL,
        steps_json TEXT NOT NULL,
        source TEXT NOT NULL,
        tested INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS dependencies (
        mod_id TEXT NOT NULL,
        name TEXT,
        version TEXT NOT NULL,
        loader TEXT,
        url TEXT,
        checksum TEXT,
        source TEXT NOT NULL,
        PRIMARY KEY (mod_id, name, version, loader, source)
    );

    CREATE INDEX IF NOT EXISTS idx_mappings_loader ON mappings(loader);
    CREATE INDEX IF NOT EXISTS idx_compatibility_loader ON compatibility(loader);
    CREATE INDEX IF NOT EXISTS idx_breakingchanges_version ON breaking_changes(version);
    CREATE INDEX IF NOT EXISTS idx_recipes_loaded ON recipes(loader);
    CREATE INDEX IF NOT EXISTS idx_dependencies_mod ON dependencies(mod_id);
    "

    /// Record schema version as applied. Use after each migration.
    pub fn record_version(&self, version: i32, description: &str) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version, description) VALUES (?1, ?2)",
            params![version, description],
        )?;
        Ok(())
    }

    /// Check whether migration to `target_version` has been applied.
    pub fn is_version_applied(&self, target_version: i32) -> Result<bool, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT version FROM schema_version WHERE version <= ?1 ORDER BY version DESC LIMIT 1",
        )?;
        let row = stmt.query_row(params![target_version], |row| row.get::<_, i32>(0));
        Ok(row.is_ok())
    }

    // --- MAPPINGS ---

    pub fn insert_mapping(&self, m: &Mapping) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO mappings (version, loader, from_api, to_api, kind, source, confidence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                m.version,
                m.loader,
                m.from_api,
                m.to_api,
                m.kind,
                m.source,
                m.confidence,
            ],
        )?;
        Ok(())
    }

    pub fn mappings_by_loader(&self, loader: &str) -> Result<Vec<Mapping>, DbError> {
        let mut stmt = self.conn.prepare("SELECT version, loader, from_api, to_api, kind, source, confidence FROM mappings WHERE loader = ?1")?;
        let rows: Vec<Mapping> = stmt.query_map(params![loader], |row| {
            Ok(Mapping {
                version: row.get(0)?,
                loader: row.get(1)?,
                from_api: row.get(2)?,
                to_api: row.get(3)?,
                kind: row.get(4)?,
                source: row.get(5)?,
                confidence: row.get(6)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
        Ok(rows)
    }

    // --- COMPATIBILITY ---

    pub fn insert_compatibility(&self, c: &Compatibility) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO compatibility (minecraft_version, loader, loader_version, status, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                c.minecraft_version,
                c.loader,
                c.loader_version,
                c.status,
                c.notes,
            ],
        )?;
        Ok(())
    }

    pub fn compatibilities_by_loader(&self, loader: &str) -> Result<Vec<Compatibility>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT minecraft_version, loader, loader_version, status, notes FROM compatibility WHERE loader = ?1",
        )?;
        let rows: Vec<Compatibility> = stmt
            .query_map(params![loader], |row| {
                Ok(Compatibility {
                    minecraft_version: row.get(0)?,
                    loader: row.get(1)?,
                    loader_version: row.get(2)?,
                    status: row.get(3)?,
                    notes: row.get(4)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    // --- BREAKING CHANGES ---

    pub fn insert_breaking_change(&self, b: &BreakingChange) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO breaking_changes (version, change, severity, workaround, source)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![b.version, b.change, b.severity, b.workaround, b.source],
        )?;
        Ok(())
    }

    pub fn breaking_changes_by_version(&self, version: &str) -> Result<Vec<BreakingChange>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT version, change, severity, workaround, source FROM breaking_changes WHERE version = ?1",
        )?;
        let rows: Vec<BreakingChange> = stmt
            .query_map(params![version], |row| {
                Ok(BreakingChange {
                    version: row.get(0)?,
                    change: row.get(1)?,
                    severity: row.get(2)?,
                    workaround: row.get(3)?,
                    source: row.get(4)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    // --- RECIPES ---

    pub fn insert_recipe(&self, r: &Recipe) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO recipes (recipe_id, version, loader, description, steps_json, source, tested)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                r.recipe_id,
                r.version,
                r.loader,
                r.description,
                serde_json::to_string(&r.steps).unwrap_or_else(|_| "[]".to_string()),
                r.source,
                if r.tested { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    pub fn recipe(&self, recipe_id: &str) -> Result<Option<Recipe>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT recipe_id, version, loader, description, steps_json, source, tested FROM recipes WHERE recipe_id = ?1",
        )?;
        let opt: Option<(String, String, Option<String>, String, String, String, i64)> = stmt
            .query_row(params![recipe_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2).ok(),
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })?;
        Ok(opt.map(|(rid, ver, loader, desc, steps_json, src, tested)| Recipe {
            recipe_id: rid,
            version: ver,
            loader,
            description: desc,
            steps: serde_json::from_str(&steps_json).unwrap_or_default(),
            source: src,
            tested: tested != 0,
        }))
    }

    pub fn recipes_by_loader(&self, loader: &str) -> Result<Vec<Recipe>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT recipe_id, version, loader, description, steps_json, source, tested FROM recipes WHERE loader = ?1",
        )?;
        let rows: Vec<Recipe> = stmt
            .query_map(params![loader], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2).ok(),
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    // --- MOD DEPENDENCIES ---

    pub fn insert_dependency(&self, d: &ModDependency) -> Result<(), DbError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO dependencies (mod_id, name, version, loader, url, checksum, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                d.mod_id,
                d.name,
                d.version,
                d.loader,
                d.url,
                d.checksum,
                d.source,
            ],
        )?;
        Ok(())
    }

    pub fn dependencies_by_mod_id(&self, mod_id: &str) -> Result<Vec<ModDependency>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT mod_id, name, version, loader, url, checksum, source FROM dependencies WHERE mod_id = ?1",
        )?;
        let rows: Vec<ModDependency> = stmt
            .query_map(params![mod_id], |row| {
                Ok(ModDependency {
                    mod_id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    loader: row.get(3).ok(),
                    url: row.get(4).ok(),
                    checksum: row.get(5).ok(),
                    source: row.get(6)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    // --- IMPORT/EXPORT JSON ---

    /// Import JSON array of mappings from file.
    pub fn import_mappings_from_file(&self, path: &Path) -> Result<(), DbError> {
        let data = std::fs::read_to_string(path)?;
        let mappings: Vec<Mapping> = serde_json::from_str(&data)?;
        for m in &mappings {
            self.insert_mapping(m)?;
        }
        Ok(())
    }

    /// Export all mappings to JSON file.
    pub fn export_mappings_to_file(&self, path: &Path) -> Result<(), DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT version, loader, from_api, to_api, kind, source, confidence FROM mappings",
        )?;
        let mappings: Vec<Mapping> = stmt
            .query_map(params![], |row| {
                Ok(Mapping {
                    version: row.get(0)?,
                    loader: row.get(1)?,
                    from_api: row.get(2)?,
                    to_api: row.get(3)?,
                    kind: row.get(4)?,
                    source: row.get(5)?,
                    confidence: row.get(6)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        let data = serde_json::to_string_pretty(&mappings)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Import JSON array of compatibilities from file.
    pub fn import_compatibilities_from_file(&self, path: &Path) -> Result<(), DbError> {
        let data = std::fs::read_to_string(path)?;
        let compatibilities: Vec<Compatibility> = serde_json::from_str(&data)?;
        for c in &compatibilities {
            self.insert_compatibility(c)?;
        }
        Ok(())
    }

    /// Export all compatibilities to JSON file.
    pub fn export_compatibilities_to_file(&self, path: &Path) -> Result<(), DbError> {
        let compatibilities = self.compatibilities_by_loader("")?;
        let data = serde_json::to_string_pretty(&compatibilities)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Import JSON array of breaking changes from file.
    pub fn import_breaking_changes_from_file(&self, path: &Path) -> Result<(), DbError> {
        let data = std::fs::read_to_string(path)?;
        let breaking_changes: Vec<BreakingChange> = serde_json::from_str(&data)?;
        for b in &breaking_changes {
            self.insert_breaking_change(b)?;
        }
        Ok(())
    }

    /// Export all breaking changes to JSON file.
    pub fn export_breaking_changes_to_file(&self, path: &Path) -> Result<(), DbError> {
        // Query all
        let mut stmt = self.conn.prepare("SELECT version, change, severity, workaround, source FROM breaking_changes")?;
        let rows: Vec<BreakingChange> = stmt
            .query_map(params![], |row| {
                Ok(BreakingChange {
                    version: row.get(0)?,
                    change: row.get(1)?,
                    severity: row.get(2)?,
                    workaround: row.get(3)?,
                    source: row.get(4)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        let data = serde_json::to_string_pretty(&rows)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Import JSON array of recipes from file.
    pub fn import_recipes_from_file(&self, path: &Path) -> Result<(), DbError> {
        let data = std::fs::read_to_string(path)?;
        let recipes: Vec<Recipe> = serde_json::from_str(&data)?;
        for r in &recipes {
            self.insert_recipe(r)?;
        }
        Ok(())
    }

    /// Export all recipes to JSON file.
    pub fn export_recipes_to_file(&self, path: &Path) -> Result<(), DbError> {
        let recipes = self.recipes_by_loader("")?;
        let data = serde_json::to_string_pretty(&recipes)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Import JSON array of mod dependencies from file.
    pub fn import_dependencies_from_file(&self, path: &Path) -> Result<(), DbError> {
        let data = std::fs::read_to_string(path)?;
        let deps: Vec<ModDependency> = serde_json::from_str(&data)?;
        for d in &deps {
            self.insert_dependency(d)?;
        }
        Ok(())
    }

    /// Export all mod dependencies to JSON file.
    pub fn export_dependencies_to_file(&self, path: &Path) -> Result<(), DbError> {
        let deps = self.dependencies_by_mod_id("")?;
        let data = serde_json::to_string_pretty(&deps)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    // --- KNOWLEDGE SEARCH (5.6) ---

    /// Search mappings where `from_api` matches `pattern` across loaders.
    pub fn search_mappings(&self, pattern: &str) -> Result<Vec<Mapping>, DbError> {
        let like = format!("%{}%", pattern);
        let mut stmt = self.conn.prepare(
            "SELECT version, loader, from_api, to_api, kind, source, confidence FROM mappings
             WHERE from_api LIKE ?1 OR to_api LIKE ?1 OR kind LIKE ?1",
        )?;
        let rows: Vec<Mapping> = stmt
            .query_map(params![like], |row| {
                Ok(Mapping {
                    version: row.get(0)?,
                    loader: row.get(1)?,
                    from_api: row.get(2)?,
                    to_api: row.get(3)?,
                    kind: row.get(4)?,
                    source: row.get(5)?,
                    confidence: row.get(6)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Search recipes where description or source matches `pattern`.
    pub fn search_recipes(&self, pattern: &str) -> Result<Vec<Recipe>, DbError> {
        let like = format!("%{}%", pattern);
        let mut stmt = self.conn.prepare(
            "SELECT recipe_id, version, loader, description, steps_json, source, tested FROM recipes
             WHERE description LIKE ?1 OR source LIKE ?1",
        )?;
        let rows: Vec<Recipe> = stmt
            .query_map(params![like], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2).ok(),
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Search breaking changes where `change` matches `pattern`.
    pub fn search_breaking_changes(&self, pattern: &str) -> Result<Vec<BreakingChange>, DbError> {
        let like = format!("%{}%", pattern);
        let mut stmt = self.conn.prepare(
            "SELECT version, change, severity, workaround, source FROM breaking_changes
             WHERE change LIKE ?1",
        )?;
        let rows: Vec<BreakingChange> = stmt
            .query_map(params![like], |row| {
                Ok(BreakingChange {
                    version: row.get(0)?,
                    change: row.get(1)?,
                    severity: row.get(2)?,
                    workaround: row.get(3)?,
                    source: row.get(4)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }
}

/// Seed data for initial population.
///
/// Contains representative mappings, compatibility entries, breaking changes,
/// and recipes for Forge/Fabric/NeoForge across Minecraft versions 1.12.2 – 1.21.
pub fn seed_data() -> Vec<(String, String)> {
    // (schema_version, description)
    vec![
        (
            1,
            "Initial schema with mappings, compatibility, breaking_changes, recipes, dependencies, schema_version tables",
        ),
    ]
}