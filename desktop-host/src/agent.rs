//! Explicit local CLI bridge. It never starts a desktop host or evaluates input.

pub fn is_action(value: &str) -> bool {
    [
        "--agent-help",
        "--agent-schema",
        "--host-schema",
        "--providers-command",
        "--get-state",
        "--validate-json",
        "--apply-json",
        "--get-result",
        "--watch-json",
        "--appearance-help",
        "--preset-command",
        "--schedule-command",
        "--focus-command",
    ]
    .contains(&value)
}

pub fn run(args: Vec<String>) -> Result<i32, String> {
    if let Some(result) = crate::cli_catalogs::get(&args) {
        return match result {
            Ok(value) => {
                println!("{value}");
                Ok(0)
            }
            Err(code) => {
                let (exit, mut value) = crate::providers::error(&code);
                if args.first().is_some_and(|a| a != "--appearance-help") {
                    value["note"] = serde_json::json!("No success claimed. Query --get-result before retrying an unknown commit outcome.");
                }
                println!("{value}");
                Ok(exit)
            }
        };
    }
    if args.first().is_some_and(|a| a == "--providers-command") {
        return Ok(crate::providers::command(&args));
    }
    if args.first().is_some_and(|a| a == "--focus-command") {
        return Ok(crate::focus::command(&args));
    }
    if args
        .first()
        .is_some_and(|a| crate::appearance_agent::is_action(a))
    {
        return Ok(crate::appearance_agent::command(&args));
    }
    if args
        .first()
        .is_some_and(|a| crate::native_appearance_cli::is_action(a))
    {
        return Ok(crate::native_appearance_cli::command(&args));
    }
    let (exit, value) = crate::providers::error("ARGUMENT_INVALID");
    println!("{value}");
    Ok(exit)
}
#[cfg(test)]
mod tests {
    #[test]
    fn explicit_commands_only() {
        assert!(super::is_action("--apply-json"));
        assert!(super::is_action("--agent-schema"));
        assert!(super::is_action("--host-schema"));
        assert!(super::is_action("--providers-command"));
        assert!(!super::is_action("--provider-url"));
        assert!(!super::is_action("--exec"));
        assert!(super::is_action("--watch-json"));
    }
}
