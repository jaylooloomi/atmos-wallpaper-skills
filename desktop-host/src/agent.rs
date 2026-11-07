//! Explicit local CLI bridge. It never starts a desktop host or evaluates input.
use std::{
    io::Write,
    os::windows::process::CommandExt,
    process::{Command, Stdio},
};

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
    if args.first().is_some_and(|a| matches!(a.as_str(), "--preset-command" | "--schedule-command")) {
        return run_legacy(args);
    }
    let (exit, value) = crate::providers::error("ARGUMENT_INVALID");
    println!("{value}");
    Ok(exit)
}
fn run_legacy(args: Vec<String>) -> Result<i32, String> {
    match forward(args) {
        Ok(code) => Ok(code),
        Err(_) => {
            println!(
                "{}",
                serde_json::json!({"apiVersion":1,"ok":false,"error":{"code":"AGENT_RUNTIME_UNAVAILABLE","retryable":false},"committed":null})
            );
            Ok(1)
        }
    }
}

fn forward(args: Vec<String>) -> Result<i32, String> {
    let root = crate::installed::payload_root()?;
    crate::installed::verify(&root)?;
    let mut command = Command::new(root.join("runtime/node/node.exe"));
    command
        .arg(root.join("app/implementation-p4_2_5/src/appearance-cli.mjs"))
        .args(&args)
        .env_remove("NODE_OPTIONS")
        .env_remove("NODE_PATH")
        .env_remove("PYTHONHOME")
        .env_remove("PYTHONPATH")
        .creation_flags(0x08000000);
    let mutation = match args.first().map(String::as_str) {
        Some("--preset-command") => args
            .get(1)
            .is_some_and(|a| ["save", "apply", "delete"].contains(&a.as_str())),
        Some("--schedule-command") => {
            args.get(1).is_some_and(|a| a == "update")
        }
        _ => false,
    };
    let result = if mutation {
        use std::io::Read;
        let mut input = Vec::new();
        std::io::stdin()
            .take(16385)
            .read_to_end(&mut input)
            .map_err(|e| e.to_string())?;
        if input.len() > 16384 {
            println!(
                "{}",
                serde_json::json!({"apiVersion":1,"ok":false,"error":{"code":"BODY_TOO_LARGE"},"committed":false})
            );
            return Ok(2);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&input).map_err(|e| e.to_string())?;
        }
        child.wait_with_output().map_err(|e| e.to_string())?
    } else {
        command
            .stdin(Stdio::null())
            .output()
            .map_err(|e| e.to_string())?
    };
    // Bound output before forwarding; the protocol prints JSON, never a token.
    if result.stdout.len() > 1024 * 1024 || result.stdout.is_empty() {
        return Err("Invalid agent runtime response".into());
    }
    serde_json::from_slice::<serde_json::Value>(&result.stdout)
        .map_err(|_| "Agent response was not JSON")?;
    std::io::stdout()
        .write_all(&result.stdout)
        .map_err(|e| e.to_string())?;
    Ok(result.status.code().unwrap_or(1))
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
