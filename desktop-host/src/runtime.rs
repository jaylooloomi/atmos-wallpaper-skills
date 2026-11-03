use crate::{options::Options, windows};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    os::windows::{io::AsRawHandle, process::CommandExt},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub struct Runtime {
    pub child: Child,
    pub port: u16,
    _job: windows::OwnedHandle,
}
impl Runtime {
    pub fn start(options: &Options) -> Result<Self, String> {
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
        let log = fs::File::create(options.store.join("native-runtime.log"))
            .map_err(|e| e.to_string())?;
        // No listener or descendants may start before the host assigns the
        // child to its kill-on-close Job. EOF also closes a child in this gap.
        const GATE: &str = "import {pathToFileURL} from 'node:url'; process.stdin.setEncoding('utf8'); let input=''; await new Promise((resolve,reject)=>{process.stdin.on('data',s=>{input+=s;if(input==='start\\n')resolve();else if(input.length>6)reject(Error('Invalid host gate'));});process.stdin.once('end',()=>process.exit(0));}); await import(pathToFileURL(process.argv[1]).href);";
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
            .env("ATMOS_STORE", &options.store)
            .env("ATMOS_PYTHON", &options.python)
            .env_remove("ATMOS_TARGET_MONITOR")
            .env_remove("PORT")
            .env_remove("NODE_OPTIONS")
            .env_remove("NODE_PATH")
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
            _job: job,
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
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
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
