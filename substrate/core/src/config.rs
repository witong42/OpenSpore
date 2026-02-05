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

    #[serde(alias = "OPENROUTER_MODEL_FAST")]
    pub model_fast: Option<String>,

    #[serde(alias = "OPENROUTER_MODEL_REASONING")]
    pub model_reasoning: Option<String>,

    #[serde(skip)]
    pub project_root: std::path::PathBuf,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        // 1. Try standard dotenv discovery from current dir
        if let Err(_) = dotenvy::dotenv() {
            // 2. Fallback: Try explicitly from OPENSPORE_ROOT or ~/.openspore
            let root_name = std::env::var("OPENSPORE_ROOT").unwrap_or_else(|_| ".openspore".to_string());

            if let Ok(home) = std::env::var("HOME") {
                let path = std::path::Path::new(&home).join(&root_name).join(".env");
                if path.exists() {
                     let _ = dotenvy::from_path(&path);
                }
            }
        }

        let builder = Config::builder()
            .set_default("autonomy_enabled", false)?
            .add_source(File::with_name("openspore").required(false))
            .add_source(Environment::default());

        let mut config: Self = builder.build()?.try_deserialize()?;

        // Set project_root
        let root_name = std::env::var("OPENSPORE_ROOT").unwrap_or_else(|_| ".openspore".to_string());
        if let Ok(home) = std::env::var("HOME") {
            config.project_root = std::path::Path::new(&home).join(&root_name);
        } else {
            config.project_root = std::path::PathBuf::from(".");
        }

        Ok(config)
    }
}
