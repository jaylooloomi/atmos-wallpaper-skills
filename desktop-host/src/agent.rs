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
    if args.first().is_some_and(|a| a == "--watch-json") {
        return forward_watch(&root, args);
    }
    let appearance = args.first().is_some_and(|a| {
        [
            "--appearance-help",
            "--preset-command",
            "--schedule-command",
            "--focus-command",
            "--providers-command",
        ]
        .contains(&a.as_str())
    });
    let mut command = Command::new(root.join("runtime/node/node.exe"));
    command
        .arg(root.join(if appearance {
            "app/implementation-p4_2_5/src/appearance-cli.mjs"
        } else {
            "app/implementation-p4_2_5/src/agent-cli.mjs"
        }))
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
        Some("--schedule-command" | "--focus-command" | "--providers-command") => {
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

// Explicit streaming session only. The owned Job closes on normal exit or parent death.
fn forward_watch(root: &std::path::Path, args: Vec<String>) -> Result<i32, String> {
    use std::{
        io::{BufRead, Read},
        os::windows::io::AsRawHandle,
    };
    let mut child = Command::new(root.join("runtime/node/node.exe"))
        .arg(root.join("app/implementation-p4_2_5/src/agent-cli.mjs"))
        .args(args)
        .env_remove("NODE_OPTIONS")
        .env_remove("NODE_PATH")
        .env_remove("PYTHONHOME")
        .env_remove("PYTHONPATH")
        .env("ATMOS_AGENT_WATCH_GATE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| e.to_string())?;
    let job = match crate::windows::child_job(child.as_raw_handle() as isize) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let result = (|| -> Result<i32, String> {
        child
            .stdin
            .take()
            .ok_or("Missing watcher input")?
            .write_all(b"start\n")
            .map_err(|e| e.to_string())?;
        let mut reader =
            std::io::BufReader::new(child.stdout.take().ok_or("Missing watcher output")?);
        loop {
            let mut line = Vec::new();
            let size = (&mut reader)
                .take(1024 * 1024 + 1)
                .read_until(b'\n', &mut line)
                .map_err(|e| e.to_string())?;
            if size == 0 {
                break;
            }
            if size > 1024 * 1024 || line.last() != Some(&b'\n') {
                return Err("Invalid watcher output".into());
            }
            let value: serde_json::Value =
                serde_json::from_slice(&line).map_err(|_| "Invalid watcher JSON")?;
            if value["apiVersion"] != 1 {
                return Err("Invalid watcher protocol".into());
            }
            let mut out = std::io::stdout().lock();
            out.write_all(&line)
                .and_then(|_| out.flush())
                .map_err(|e| e.to_string())?;
        }
        Ok(child.wait().map_err(|e| e.to_string())?.code().unwrap_or(1))
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    drop(job);
    result
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
