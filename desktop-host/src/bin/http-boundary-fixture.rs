//! Feature-gated synthetic boundary executable. Never used by installed App.
#[path = "../native_http.rs"]
mod native_http;
use native_http::{ApiPolicy, Assets, Limits, Response, Server};
use std::{
    io::Read,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
fn main() {
    if let Err(e) = run() {
        eprintln!("HTTP fixture failed: {e}");
        std::process::exit(2)
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() % 2 != 0 {
        return Err("Arguments invalid".into());
    }
    let mut root = None;
    let mut duration = 30000;
    let mut headers = 10000;
    let mut body = 15000;
    let mut write = 5000;
    let mut seen = std::collections::HashSet::new();
    for pair in args.chunks(2) {
        if !seen.insert(&pair[0]) {
            return Err("Duplicate option".into());
        }
        let number = || {
            pair[1]
                .parse::<u64>()
                .map_err(|_| "Limit invalid".to_owned())
        };
        match pair[0].as_str() {
            "--root" => root = Some(PathBuf::from(&pair[1])),
            "--duration-ms" => duration = number()?,
            "--header-ms" => headers = number()?,
            "--body-ms" => body = number()?,
            "--write-ms" => write = number()?,
            _ => return Err("Unknown option".into()),
        }
    }
    if !(100..=120000).contains(&duration)
        || [headers, body, write]
            .iter()
            .any(|v| !(50..=15000).contains(v))
    {
        return Err("Limit out of range".into());
    }
    let assets = Assets::load(&root.ok_or("Root required")?).map_err(|e| e.code.to_owned())?;
    let server = Server::bind(Limits {
        headers: Duration::from_millis(headers),
        body: Duration::from_millis(body),
        write: Duration::from_millis(write),
    })
    .map_err(|e| e.code.to_owned())?;
    let stop = Arc::new(AtomicBool::new(false));
    let done = stop.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(duration));
        done.store(true, Ordering::Release)
    });
    let eof = stop.clone();
    std::thread::spawn(move || {
        let _ = std::io::stdin().read_to_end(&mut Vec::new());
        eof.store(true, Ordering::Release)
    });
    println!(
        "{}",
        serde_json::json!({"origin":server.security.origin,"syntheticOnly":true,"stores":false,"rendererAcknowledgements":false})
    );
    server.run(stop,move|security,req|{if req.target.starts_with("/api/"){if let Err(e)=security.authenticate_api(&req){return Response::error(e)}let policy=match req.target.as_str(){"/api/snapshot"=>ApiPolicy::read(),"/api/commands"=>ApiPolicy::manager_json(32768),"/api/calendar/import"=>ApiPolicy::manager_json(2*1048576),_=>return Response::json(404,serde_json::json!({"error":{"code":"API_NOT_IMPLEMENTED"}}))};match security.api(&req,policy){Err(e)=>Response::error(e),Ok(role)=>Response::json(200,serde_json::json!({"syntheticBoundaryOnly":true,"role":format!("{role:?}"),"bodyBytes":req.body.len(),"noStoreWrite":true}))}}else{security.static_response(&req,&assets).unwrap_or_else(Response::error)}}).map_err(|e|e.code.to_owned())
}
