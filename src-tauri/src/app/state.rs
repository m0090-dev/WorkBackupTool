use crate::core::config::assets::*;
use crate::core::types::AppConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub config_path: PathBuf,
    pub i18n: HashMap<String, HashMap<String, String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn save(&self) -> Result<(), String> {
        let cfg = self.config.lock().unwrap();
        let data = serde_json::to_string_pretty(&*cfg).map_err(|e| e.to_string())?;
        fs::write(&self.config_path, data).map_err(|e: std::io::Error| e.to_string())?;
        Ok(())
    }
    pub fn translate(&self, key: &str) -> Result<String, String> {
        let cfg = self.config.lock().unwrap();
        let lang = {
            if cfg.language.is_empty() {
                "ja".to_string()
            } else {
                cfg.language.clone()
            }
        };
        self.i18n
            .get(&lang)
            .and_then(|m| m.get(key))
            .cloned()
            .ok_or_else(|| key.to_string())
    }
}

impl Default for AppState {
    fn default() -> Self {
        // 既存のヘルパー関数的なロジックをインライン化、または利用
        let config: AppConfig =
            serde_json::from_str(DEFAULT_CONFIG_JSON).expect("Embedded AppConfig.json is invalid");

        let i18n: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(DEFAULT_I18N_JSON).unwrap_or_default();

        Self {
            config: Mutex::new(config),
            config_path: PathBuf::new(), // テスト時は空、load_app_config 時に上書き
            i18n,
        }
    }
}
