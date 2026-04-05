use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

static DOTENV: OnceLock<()> = OnceLock::new();
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Credentials {
    pub api_key: String,
    pub secret_key: String,
}

pub fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn repo_root_dotenv_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env")
}

fn read_trimmed_env(name: &str) -> Option<String> {
    let value = std::env::var(name).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_owned())
}

pub fn trade_credentials() -> Option<Credentials> {
    DOTENV.get_or_init(|| {
        let _ = dotenvy::from_path(repo_root_dotenv_path());
    });

    let api_key = read_trimmed_env("ALPACA_TRADE_API_KEY")?;
    let secret_key = read_trimmed_env("ALPACA_TRADE_SECRET_KEY")?;

    Some(Credentials {
        api_key,
        secret_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_trimmed_env_rejects_blank_values() {
        let _env_guard = env_lock();

        unsafe {
            std::env::set_var("ALPACA_TRADE_API_KEY", "   ");
        }

        assert_eq!(read_trimmed_env("ALPACA_TRADE_API_KEY"), None);

        unsafe {
            std::env::remove_var("ALPACA_TRADE_API_KEY");
        }
    }

    #[test]
    fn read_trimmed_env_trims_non_blank_values() {
        let _env_guard = env_lock();

        unsafe {
            std::env::set_var("ALPACA_TRADE_SECRET_KEY", "  secret  ");
        }

        assert_eq!(
            read_trimmed_env("ALPACA_TRADE_SECRET_KEY").as_deref(),
            Some("secret")
        );

        unsafe {
            std::env::remove_var("ALPACA_TRADE_SECRET_KEY");
        }
    }

    #[test]
    fn repo_root_dotenv_path_points_at_workspace_root() {
        let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");

        assert_eq!(repo_root_dotenv_path(), expected);
    }
}
