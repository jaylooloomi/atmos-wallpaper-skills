use std::path::PathBuf;

#[derive(Debug)]
pub struct Options {
    pub runtime_root: PathBuf,
    pub node: PathBuf,
    pub python: PathBuf,
    pub store: PathBuf,
    pub status: PathBuf,
    pub seconds: Option<u64>,
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut values = std::collections::BTreeMap::new();
    let mut args = args.into_iter();
    while let Some(key) = args.next() {
        if ![
            "--runtime-root",
            "--node",
            "--python",
            "--store",
            "--status",
            "--run-seconds",
        ]
        .contains(&key.as_str())
        {
            return Err(format!("Unknown option: {key}"));
        }
        let value = args.next().ok_or(format!("Missing value for {key}"))?;
        if values.insert(key.clone(), value).is_some() {
            return Err(format!("Duplicate option: {key}"));
        }
    }
    let get_path = |key: &str| -> Result<PathBuf, String> {
        let p = PathBuf::from(values.get(key).ok_or(format!("Required option: {key}"))?);
        if !p.is_absolute() {
            return Err(format!("{key} must be absolute"));
        }
        Ok(p)
    };
    let seconds = values
        .get("--run-seconds")
        .map(|v| v.parse::<u64>())
        .transpose()
        .map_err(|_| "Invalid duration")?
        .unwrap_or(600);
    if !(5..=3600).contains(&seconds) {
        return Err("Duration must be 5..3600 seconds".into());
    }
    Ok(Options {
        runtime_root: get_path("--runtime-root")?,
        node: get_path("--node")?,
        python: get_path("--python")?,
        store: get_path("--store")?,
        status: get_path("--status")?,
        seconds: Some(seconds),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args() -> Vec<String> {
        [
            "--runtime-root",
            "C:\\app",
            "--node",
            "C:\\node.exe",
            "--python",
            "C:\\python.exe",
            "--store",
            "C:\\store",
            "--status",
            "C:\\run\\status.json",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }
    #[test]
    fn requires_explicit_paths() {
        assert!(parse(Vec::<String>::new()).is_err());
    }
    #[test]
    fn bounds_duration() {
        let mut a = args();
        a.extend(["--run-seconds".into(), "0".into()]);
        assert!(parse(a).is_err());
    }
    #[test]
    fn rejects_duplicates() {
        let mut a = args();
        a.extend(["--store".into(), "C:\\other".into()]);
        assert!(parse(a).is_err());
    }
    #[test]
    fn rejects_unknown_options() {
        let mut a = args();
        a.push("--url".into());
        assert!(parse(a).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn valid_explicit_configuration() {
        assert_eq!(parse(args()).unwrap().seconds, Some(600));
    }
}
