//! Static contracts are data embedded in the native executable, not JS programs.
use serde_json::Value;

pub fn get(args: &[String]) -> Option<Result<Value, String>> {
    let name = args.first()?.as_str();
    let text = match name {
        "--agent-help" => include_str!("cli-catalogs/agent-help.json"),
        "--agent-schema" => include_str!("cli-catalogs/agent-schema.json"),
        "--host-schema" => include_str!("cli-catalogs/host-schema.json"),
        "--appearance-help" => include_str!("cli-catalogs/appearance-help.json"),
        _ => return None,
    };
    // Agent's catalog parser accepts an unused absolute --store option. The
    // appearance help command accepts no options. Never create/read that store.
    let valid = args.len() == 1
        || (name != "--appearance-help"
            && args.len() == 3
            && args[1] == "--store"
            && crate::portable_store::node_absolute(&args[2]));
    Some(if valid {
        serde_json::from_str(text).map_err(|_| "CLI_CATALOG_INVALID".into())
    } else {
        Err("ARGUMENT_INVALID".into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogs_are_embedded_without_runtime_or_store() {
        for command in [
            "--agent-help",
            "--agent-schema",
            "--host-schema",
            "--appearance-help",
        ] {
            assert!(get(&[command.into()]).unwrap().unwrap().is_object());
            assert_eq!(
                get(&[command.into(), "--exec".into()])
                    .unwrap()
                    .unwrap_err(),
                "ARGUMENT_INVALID"
            );
        }
        assert!(get(&["--get-state".into()]).is_none());
    }
}
