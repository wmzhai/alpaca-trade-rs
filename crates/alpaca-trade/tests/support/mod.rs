#[allow(dead_code)]
pub mod http_server;
#[allow(unused_imports)]
pub use http_server::TestServer;

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

static DOTENV: OnceLock<HashMap<String, String>> = OnceLock::new();
const API_KEY_CANDIDATES: [&str; 2] = ["ALPACA_TRADE_API_KEY", "APCA_API_KEY_ID"];
const SECRET_KEY_CANDIDATES: [&str; 2] = ["ALPACA_TRADE_SECRET_KEY", "APCA_API_SECRET_KEY"];

#[derive(Debug, Clone)]
pub struct Credentials {
    pub api_key: String,
    pub secret_key: String,
}

fn repo_root_dotenv_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env")
}

fn normalized_value(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_owned())
}

fn read_dotenv_file(path: &Path) -> HashMap<String, String> {
    let Ok(iter) = dotenvy::from_path_iter(path) else {
        return HashMap::new();
    };

    iter.filter_map(Result::ok)
        .filter_map(|(name, value)| normalized_value(Some(&value)).map(|value| (name, value)))
        .collect()
}

fn dotenv_values() -> &'static HashMap<String, String> {
    DOTENV.get_or_init(|| read_dotenv_file(&repo_root_dotenv_path()))
}

fn select_credential(
    name: &str,
    process_value: Option<&str>,
    dotenv_values: &HashMap<String, String>,
) -> Option<String> {
    normalized_value(process_value)
        .or_else(|| normalized_value(dotenv_values.get(name).map(String::as_str)))
}

fn select_credential_candidates(
    names: &[&str],
    process_values: &HashMap<String, String>,
    dotenv_values: &HashMap<String, String>,
) -> Option<String> {
    names
        .iter()
        .find_map(|name| normalized_value(process_values.get(*name).map(String::as_str)))
        .or_else(|| {
            names
                .iter()
                .find_map(|name| normalized_value(dotenv_values.get(*name).map(String::as_str)))
        })
}

pub fn trade_credentials() -> Option<Credentials> {
    let process_values = API_KEY_CANDIDATES
        .iter()
        .chain(SECRET_KEY_CANDIDATES.iter())
        .filter_map(|name| {
            normalized_value(std::env::var(name).ok().as_deref())
                .map(|value| ((*name).to_owned(), value))
        })
        .collect::<HashMap<_, _>>();
    let dotenv_values = dotenv_values();

    let api_key =
        select_credential_candidates(&API_KEY_CANDIDATES, &process_values, dotenv_values)?;
    let secret_key =
        select_credential_candidates(&SECRET_KEY_CANDIDATES, &process_values, dotenv_values)?;

    Some(Credentials {
        api_key,
        secret_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn normalized_value_rejects_blank_values() {
        assert_eq!(normalized_value(Some("   ")), None);
    }

    #[test]
    fn normalized_value_trims_non_blank_values() {
        assert_eq!(
            normalized_value(Some("  secret  ")).as_deref(),
            Some("secret")
        );
    }

    #[test]
    fn select_credential_prefers_trimmed_process_value() {
        let dotenv = HashMap::from([("ALPACA_TRADE_API_KEY".to_owned(), "dotenv-key".to_owned())]);

        assert_eq!(
            select_credential("ALPACA_TRADE_API_KEY", Some("  process-key  "), &dotenv).as_deref(),
            Some("process-key")
        );
    }

    #[test]
    fn select_credential_falls_back_to_trimmed_dotenv_value() {
        let dotenv = HashMap::from([(
            "ALPACA_TRADE_SECRET_KEY".to_owned(),
            "  dotenv-secret  ".to_owned(),
        )]);

        assert_eq!(
            select_credential("ALPACA_TRADE_SECRET_KEY", Some("   "), &dotenv).as_deref(),
            Some("dotenv-secret")
        );
    }

    #[test]
    fn select_credential_candidates_falls_back_to_standard_process_env_name() {
        let process =
            HashMap::from([("APCA_API_KEY_ID".to_owned(), "  official-key  ".to_owned())]);
        let dotenv = HashMap::new();

        assert_eq!(
            select_credential_candidates(
                &["ALPACA_TRADE_API_KEY", "APCA_API_KEY_ID"],
                &process,
                &dotenv,
            )
            .as_deref(),
            Some("official-key")
        );
    }

    #[test]
    fn select_credential_candidates_favors_process_env_over_dotenv_aliases() {
        let process = HashMap::from([(
            "APCA_API_SECRET_KEY".to_owned(),
            "process-secret".to_owned(),
        )]);
        let dotenv = HashMap::from([(
            "ALPACA_TRADE_SECRET_KEY".to_owned(),
            "dotenv-secret".to_owned(),
        )]);

        assert_eq!(
            select_credential_candidates(
                &["ALPACA_TRADE_SECRET_KEY", "APCA_API_SECRET_KEY"],
                &process,
                &dotenv,
            )
            .as_deref(),
            Some("process-secret")
        );
    }

    #[test]
    fn select_credential_candidates_falls_back_to_standard_dotenv_name() {
        let process = HashMap::new();
        let dotenv = HashMap::from([(
            "APCA_API_SECRET_KEY".to_owned(),
            "  dotenv-secret  ".to_owned(),
        )]);

        assert_eq!(
            select_credential_candidates(
                &["ALPACA_TRADE_SECRET_KEY", "APCA_API_SECRET_KEY"],
                &process,
                &dotenv,
            )
            .as_deref(),
            Some("dotenv-secret")
        );
    }

    #[test]
    fn read_dotenv_file_parses_without_touching_process_env() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("alpaca-trade-dotenv-{unique}.env"));
        fs::write(
            &path,
            "ALPACA_TRADE_API_KEY=dotenv-key\nALPACA_TRADE_SECRET_KEY= dotenv-secret \n",
        )
        .expect("temp dotenv should write");

        let values = read_dotenv_file(&path);
        fs::remove_file(&path).expect("temp dotenv should remove");

        assert_eq!(
            values.get("ALPACA_TRADE_API_KEY").map(String::as_str),
            Some("dotenv-key")
        );
        assert_eq!(
            values.get("ALPACA_TRADE_SECRET_KEY").map(String::as_str),
            Some("dotenv-secret")
        );
    }

    #[test]
    fn repo_root_dotenv_path_points_at_workspace_root() {
        let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env");

        assert_eq!(repo_root_dotenv_path(), expected);
    }
}
