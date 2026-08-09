//! Versioned TOML workspace configuration: parse, validate, atomic write.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::domain::{CONFIG_SCHEMA_VERSION, Config};
use crate::error::AwcError;

/// Config file name inside the `.awc` directory.
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// Parses and validates config bytes.
///
/// Invalid TOML or a malformed schema yields [`AwcError::InvalidConfig`]; a
/// config that parses but declares an unsupported `schema_version` yields
/// [`AwcError::UnsupportedConfigVersion`]. Callers must keep valid bytes
/// untouched and re-serialize only to create a brand-new default config.
pub fn parse_config(bytes: &[u8]) -> Result<Config, AwcError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|err| AwcError::InvalidConfig(format!("not valid UTF-8: {err}")))?;
    let config: Config =
        toml::from_str(text).map_err(|err| AwcError::InvalidConfig(err.to_string()))?;
    if config.schema_version != CONFIG_SCHEMA_VERSION {
        return Err(AwcError::UnsupportedConfigVersion(config.schema_version));
    }
    Ok(config)
}

/// Serialized bytes for a fresh default config (`schema_version = 1`).
pub fn default_config_bytes() -> Vec<u8> {
    let mut text = toml::to_string(&Config::default_config())
        .expect("serializing the fixed default config cannot fail");
    text.push('\n');
    text.into_bytes()
}

/// Atomically writes `bytes` to `<dir>/config.toml` via a same-directory
/// temporary file and rename; on failure the temporary file is removed.
pub fn write_config_atomic(dir: &Path, bytes: &[u8]) -> Result<(), AwcError> {
    let path = dir.join(CONFIG_FILE_NAME);
    let tmp = dir.join(format!(".{}.tmp{}", CONFIG_FILE_NAME, std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map_err(AwcError::Io)
}

/// Reads and parses an existing config without creating anything. Read-only
/// status/doctor use this so checking a workspace can never write a config.
pub fn load_readonly(dir: &Path) -> Result<Config, AwcError> {
    let path = dir.join(CONFIG_FILE_NAME);
    match fs::read(&path) {
        Ok(bytes) => parse_config(&bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(AwcError::InvalidConfig(
            "config.toml is missing".to_string(),
        )),
        Err(err) => Err(AwcError::Io(err)),
    }
}

/// Loads the workspace config, atomically creating the default when absent.
///
/// Existing valid config bytes are preserved untouched; invalid TOML and
/// unsupported `schema_version` values are rejected.
pub fn load_or_create(dir: &Path) -> Result<Config, AwcError> {
    let path = dir.join(CONFIG_FILE_NAME);
    match fs::read(&path) {
        Ok(bytes) => parse_config(&bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            write_config_atomic(dir, &default_config_bytes())?;
            Ok(Config::default_config())
        }
        Err(err) => Err(AwcError::Io(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("awc-core-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn invalid_toml_is_rejected() {
        let err = parse_config(b"schema_version = [not").unwrap_err();
        assert!(matches!(err, AwcError::InvalidConfig(_)));
    }

    #[test]
    fn missing_fields_are_invalid() {
        let err = parse_config(b"schema_version = 1\n").unwrap_err();
        assert!(matches!(err, AwcError::InvalidConfig(_)));
    }

    #[test]
    fn unknown_schema_version_is_rejected() {
        for version in [0u32, 2, 99] {
            let text = format!("schema_version = {version}\ndatabase_file = \"state.sqlite3\"\n");
            let err = parse_config(text.as_bytes()).unwrap_err();
            assert!(matches!(err, AwcError::UnsupportedConfigVersion(v) if v == version));
        }
    }

    #[test]
    fn valid_config_with_comments_parses_and_keeps_bytes() {
        let bytes = b"# local override\nschema_version = 1\ndatabase_file = \"state.sqlite3\"\n";
        let config = parse_config(bytes).unwrap();
        assert_eq!(config.schema_version, 1);
        assert_eq!(config.database_file, "state.sqlite3");

        let dir = temp_dir("preserve");
        let path = dir.join(CONFIG_FILE_NAME);
        fs::write(&path, bytes).unwrap();
        load_or_create(&dir).unwrap();
        assert_eq!(
            fs::read(&path).unwrap(),
            bytes,
            "valid bytes must be preserved"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn atomic_write_round_trips_exact_bytes() {
        let dir = temp_dir("roundtrip");
        let bytes = b"schema_version = 1\ndatabase_file = \"state.sqlite3\"\n";
        write_config_atomic(&dir, bytes).unwrap();
        assert_eq!(fs::read(dir.join(CONFIG_FILE_NAME)).unwrap(), bytes);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_config_creates_default_atomically() {
        let dir = temp_dir("create");
        let config = load_or_create(&dir).unwrap();
        assert_eq!(config, Config::default_config());
        let bytes = fs::read(dir.join(CONFIG_FILE_NAME)).unwrap();
        assert_eq!(bytes, default_config_bytes());
        // A second load keeps the file byte-identical.
        assert_eq!(load_or_create(&dir).unwrap(), config);
        assert_eq!(fs::read(dir.join(CONFIG_FILE_NAME)).unwrap(), bytes);
        fs::remove_dir_all(&dir).ok();
    }
}
