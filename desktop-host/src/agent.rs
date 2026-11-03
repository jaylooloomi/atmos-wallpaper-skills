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
        "--get-state",
        "--validate-json",
        "--apply-json",
        "--get-result",
    ]
    .contains(&value)
}

pub fn run(args: Vec<String>) -> Result<i32, String> {
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
    let result = Command::new(root.join("runtime/node/node.exe"))
        .arg(root.join("app/implementation-p4_2_5/src/agent-cli.mjs"))
        .args(args)
        .env_remove("NODE_OPTIONS")
        .env_remove("NODE_PATH")
        .env_remove("PYTHONHOME")
        .env_remove("PYTHONPATH")
        .stdin(Stdio::null())
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
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
        assert!(!super::is_action("--exec"));
        assert!(!super::is_action("--watch-json"));
    }
}
