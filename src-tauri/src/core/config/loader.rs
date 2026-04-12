use crate::core::types::AppConfig;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn default_config() -> AppConfig {
    serde_json::from_str(DEFAULT_CONFIG_JSON)
        .expect("Embedded AppConfig.json is invalid. Please check the JSON format at compile time.")
}
pub fn default_i18n() -> HashMap<String, HashMap<String, String>> {
    serde_json::from_str(DEFAULT_I18N_JSON).unwrap_or_default()
}




pub fn load_app_config(config_path: PathBuf) -> Result<AppConfig, String> {
    // 1. 親ディレクトリ（AppConfigDir）がなければ作成
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }

    // 2. ファイルの読み込み、またはデフォルトの書き込み
    let data = if config_path.exists() {
        fs::read_to_string(&config_path).map_err(|e| e.to_string())?
    } else {
        fs::write(&config_path, super::DEFAULT_CONFIG_JSON).map_err(|e| e.to_string())?;
        crate::core::config::DEFAULT_CONFIG_JSON.to_string()
    };

    // 3. デシリアライズ
    let cfg: AppConfig = serde_json::from_str(&data).map_err(|e| e.to_string())?;

    Ok(cfg)
}


