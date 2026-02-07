use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(alias = "OPENROUTER_API_KEY")]
    pub openrouter_api_key: String,

    #[serde(alias = "TELEGRAM_BOT_TOKEN")]
    pub telegram_bot_token: Option<String>,

    #[serde(alias = "TELEGRAM_ALLOWED_USERS")]
    pub telegram_allowed_users: Option<String>,

    #[serde(alias = "AUTONOMY_ENABLED")]
    pub autonomy_enabled: bool,

    #[serde(alias = "OPENROUTER_MODEL")]
    pub model: Option<String>,

    #[serde(alias = "SAFE_MODE_ENABLED")]
    pub safe_mode_enabled: bool,

    #[serde(skip)]
    pub project_root: std::path::PathBuf,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        // 1. Try standard dotenv discovery from current dir
        if let Err(_) = dotenvy::dotenv() {
            // 2. Fallback: Try explicitly from resolved OPENSPORE_ROOT
            let root = crate::path_utils::get_app_root();
            let path = root.join(".env");
            if path.exists() {
                 let _ = dotenvy::from_path(&path);
            }
        }

        let builder = Config::builder()
            .set_default("autonomy_enabled", false)?
            .set_default("safe_mode_enabled", false)?
            .add_source(File::with_name("openspore").required(false))
            .add_source(Environment::default());

        let mut config: Self = builder.build()?.try_deserialize()?;

        // Set project_root
        config.project_root = crate::path_utils::get_app_root();

        Ok(config)
    }
}
