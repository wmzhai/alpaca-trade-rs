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

    struct EnvVarRestore {
        name: &'static str,
        original: Option<String>,
    }

    impl EnvVarRestore {
        fn set(name: &'static str, value: &str) -> Self {
            let original = std::env::var(name).ok();
            unsafe {
                std::env::set_var(name, value);
            }
            Self { name, original }
        }
    }

    impl Drop for EnvVarRestore {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => unsafe {
                    std::env::set_var(self.name, value);
                },
                None => unsafe {
                    std::env::remove_var(self.name);
                },
            }
        }
    }

    #[test]
    fn read_trimmed_env_rejects_blank_values() {
        let _env_guard = env_lock();
        let _restore = EnvVarRestore::set("ALPACA_TRADE_API_KEY", "   ");

        assert_eq!(read_trimmed_env("ALPACA_TRADE_API_KEY"), None);
    }

    #[test]
    fn read_trimmed_env_trims_non_blank_values() {
        let _env_guard = env_lock();
        let _restore = EnvVarRestore::set("ALPACA_TRADE_SECRET_KEY", "  secret  ");

        assert_eq!(
            read_trimmed_env("ALPACA_TRADE_SECRET_KEY").as_deref(),
            Some("secret")
        );
    }

    #[test]
    fn env_var_restore_restores_prior_value() {
        let _env_guard = env_lock();
        unsafe {
            std::env::set_var("ALPACA_TRADE_API_KEY", "original");
        }

        {
            let _restore = EnvVarRestore::set("ALPACA_TRADE_API_KEY", "temporary");
            assert_eq!(std::env::var("ALPACA_TRADE_API_KEY").ok().as_deref(), Some("temporary"));
        }

        assert_eq!(std::env::var("ALPACA_TRADE_API_KEY").ok().as_deref(), Some("original"));

        unsafe {
            std::env::remove_var("ALPACA_TRADE_API_KEY");
        }
    }

    #[test]
    fn repo_root_dotenv_path_points_at_workspace_root() {
        let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");

        assert_eq!(repo_root_dotenv_path(), expected);
    }
}
