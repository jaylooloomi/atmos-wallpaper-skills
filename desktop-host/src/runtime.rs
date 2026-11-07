use crate::{options::Options, windows};
#[cfg(not(feature = "native-runtime"))]
use std::net::{TcpListener, TcpStream};
use std::{
    fs,
    io::{Read, Write},
    os::windows::{io::AsRawHandle, process::CommandExt},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
#[cfg(feature = "native-core")]
use std::{
    io::{BufRead, BufReader},
    path::Path,
};

pub struct Runtime {
    pub child: Child,
    pub port: u16,
    pub session_id: String,
    _job: windows::OwnedHandle,
    #[cfg(feature = "native-core")]
    graceful: bool,
}
impl Runtime {
    #[cfg(feature = "native-core")]
    pub(crate) fn stop_native(&mut self) -> bool {
        drop(self.child.stdin.take());
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                Ok(Some(status)) => return status.success(),
                Err(_) => break,
            }
        }
        false
    }

    #[cfg(not(feature = "native-runtime"))]
    pub fn start(options: &Options) -> Result<Self, String> {
        Self::start_with_mode(options, "wallpaper")
    }
    #[cfg(feature = "native-runtime")]
    pub fn start(options: &Options) -> Result<Self, String> {
        let app = options
            .runtime_root
            .parent()
            .ok_or("NATIVE_APP_ROOT_INVALID")?;
        Self::start_native(
            &options.store,
            &options.runtime_root.join("dist"),
            &app.join("prototype-v8.1-frozen"),
            &crate::backup_image::Decoder {
                python: options.python.clone(),
                script: app.join("implementation-p1/src/decode-image.py"),
            },
            None,
        )
    }

    #[cfg(feature = "native-core")]
    pub(crate) fn start_native(
        root: &Path,
        assets: &Path,
        prototype: &Path,
        decoder: &crate::backup_image::Decoder,
        topology: Option<&Path>,
    ) -> Result<Self, String> {
        crate::portable_store::validate_root(root)?;
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let log = fs::File::create(root.join("native-runtime.log")).map_err(|e| e.to_string())?;
        let session_id = format!("{}-{}", std::process::id(), crate::journal::now_ms());
        let mut command = Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        command
            .arg("--native-runtime-worker")
            .args([root, assets, prototype, &decoder.python, &decoder.script])
            .arg(&session_id)
            .args(topology);
        let mut child = command
            .env_remove("NODE_OPTIONS")
            .env_remove("NODE_PATH")
            .env_remove("PYTHONHOME")
            .env_remove("PYTHONPATH")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log))
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        let job = match windows::child_job(child.as_raw_handle() as isize) {
            Ok(job) => job,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let mut runtime = Self {
            child,
            port: 0,
            session_id,
            _job: job,
            graceful: true,
        };
        let stdout = runtime
            .child
            .stdout
            .take()
            .ok_or("NATIVE_READY_PIPE_MISSING")?;
        let (send, ready) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = BufReader::new(stdout.take(4097)).read_until(b'\n', &mut bytes);
            if result.is_ok() && bytes.len() <= 4096 && bytes.ends_with(b"\n") {
                let _ = send.send(bytes);
            }
        });
        runtime
            .child
            .stdin
            .as_mut()
            .ok_or("NATIVE_GATE_PIPE_MISSING")?
            .write_all(b"start\n")
            .map_err(|e| e.to_string())?;
        let bytes = ready
            .recv_timeout(Duration::from_secs(20))
            .map_err(|_| "Native runtime readiness failed; see native-runtime.log")?;
        let ready: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "NATIVE_READY_INVALID")?;
        runtime.port = ready["origin"]
            .as_str()
            .and_then(|origin| origin.strip_prefix("http://127.0.0.1:"))
            .and_then(|port| port.parse::<u16>().ok())
            .filter(|port| *port > 0)
            .ok_or("NATIVE_READY_INVALID")?;
        if ready["sessionId"].as_str() != Some(runtime.session_id.as_str())
            || runtime
                .child
                .try_wait()
                .map_err(|e| e.to_string())?
                .is_some()
        {
            return Err("NATIVE_READY_INVALID".into());
        }
        Ok(runtime)
    }

    #[cfg(not(feature = "native-runtime"))]
    pub fn start_with_mode(options: &Options, presentation: &str) -> Result<Self, String> {
        if presentation != "wallpaper" {
            return Err("Invalid presentation mode".into());
        }
        for p in [
            &options.node,
            &options.python,
            &options.runtime_root.join("dist/index.html"),
            &options.runtime_root.join("src/windows-runtime.mjs"),
        ] {
            if !p.is_file() {
                return Err(format!("Missing required file: {}", p.display()));
            }
        }
        // A separate store avoids changing an existing manager's settings or
        // calendar. A persisted prototype store can be reused on later runs.
        fs::create_dir_all(&options.store).map_err(|e| e.to_string())?;
        let port = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| e.to_string())?
            .local_addr()
            .map_err(|e| e.to_string())?
            .port();
        let log = fs::File::create(options.store.join(if presentation == "screensaver" {
            format!("screensaver-runtime-{}.log", std::process::id())
        } else {
            "native-runtime.log".into()
        }))
        .map_err(|e| e.to_string())?;
        // No listener or descendants may start before the host assigns the
        // child to its kill-on-close Job. EOF also closes a child in this gap.
        const GATE: &str = "import {pathToFileURL} from 'node:url'; process.stdin.setEncoding('utf8'); let input=''; await new Promise((resolve,reject)=>{process.stdin.on('data',s=>{input+=s;if(input==='start\\n')resolve();else if(input.length>6)reject(Error('Invalid host gate'));});process.stdin.once('end',()=>process.exit(0));}); await import(pathToFileURL(process.argv[1]).href);";
        let session_id = format!("{}-{}", std::process::id(), crate::journal::now_ms());
        let mut child = Command::new(&options.node)
            .args(["--input-type=module", "-e", GATE])
            .arg(options.runtime_root.join("src/windows-runtime.mjs"))
            .args([
                "--start-local-service",
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--strictPort",
            ])
            .current_dir(&options.runtime_root)
            .env("ATMOS_HOST_SESSION_ID", &session_id)
            .env("ATMOS_PRESENTATION", presentation)
            .env("ATMOS_STORE", &options.store)
            .env("ATMOS_DESKTOP_HOST_STORE", &options.store)
            .env("ATMOS_PYTHON", &options.python)
            .env(
                "ATMOS_DESKTOP_HOST_EXE",
                std::env::current_exe().map_err(|e| e.to_string())?,
            )
            .env_remove("ATMOS_TARGET_MONITOR")
            .env_remove("PORT")
            .env_remove("NODE_OPTIONS")
            .env_remove("NODE_PATH")
            .env_remove("PYTHONHOME")
            .env_remove("PYTHONPATH")
            .stdin(Stdio::piped())
            .stdout(Stdio::from(log.try_clone().map_err(|e| e.to_string())?))
            .stderr(Stdio::from(log))
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        let job = match windows::child_job(child.as_raw_handle() as isize) {
            Ok(v) => v,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e);
            }
        };
        if let Err(e) = child.stdin.as_mut().unwrap().write_all(b"start\n") {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e.to_string());
        }
        let mut runtime = Self {
            child,
            port,
            session_id,
            _job: job,
            #[cfg(feature = "native-core")]
            graceful: false,
        };
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(20) {
            if runtime
                .child
                .try_wait()
                .map_err(|e| e.to_string())?
                .is_some()
            {
                return Err("Local runtime exited before readiness; see native-runtime.log".into());
            }
            if ready(port) {
                return Ok(runtime);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err("Local runtime readiness timed out".into())
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        #[cfg(feature = "native-core")]
        if self.graceful && self.stop_native() {
            return;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn check_embedded(root: &std::path::Path) -> Result<(), String> {
    #[cfg(not(feature = "native-runtime"))]
    {
        let node = Command::new(root.join("runtime/node/node.exe"))
            .arg("--version")
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        if !node.status.success() {
            return Err("Bundled Node self-check failed".into());
        }
    }
    #[cfg(feature = "native-runtime")]
    crate::native_http::Assets::load(&root.join("app/implementation-p4_2_5/dist"))
        .map_err(|error| error.code)?;
    let python = Command::new(root.join("runtime/python/python.exe"))
        .args([
            "-I",
            "-B",
            "-c",
            "from PIL import Image,features;assert features.check('webp')",
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;
    if !python.status.success() {
        return Err("Bundled Python/Pillow self-check failed".into());
    }
    Ok(())
}
#[cfg(not(feature = "native-runtime"))]
fn ready(port: u16) -> bool {
    let address = ([127, 0, 0, 1], port).into();
    let Ok(mut socket) = TcpStream::connect_timeout(&address, Duration::from_millis(300)) else {
        return false;
    };
    let _ = socket.set_read_timeout(Some(Duration::from_millis(300)));
    if write!(
        socket,
        "GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .is_err()
    {
        return false;
    }
    let mut b = [0; 128];
    let n = socket.read(&mut b).unwrap_or(0);
    String::from_utf8_lossy(&b[..n]).starts_with("HTTP/1.1 200")
}
