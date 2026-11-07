//! User-invoked native folder selection only; no browser-granted arbitrary paths.
use std::{
    io::Read,
    path::PathBuf,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Storage::FileSystem::GetDriveTypeW,
    System::Com::{
        CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
        COINIT_DISABLE_OLE1DDE,
    },
    System::WindowsProgramming::{DRIVE_FIXED, DRIVE_REMOVABLE},
    UI::Shell::*,
};
pub(crate) fn local_path(s: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(s);
    if !p.is_absolute() || s.starts_with(r"\\") || s.contains(['\0', '\r', '\n']) || s.len() > 30000
    {
        return Err("LIBRARY_PATH_UNSAFE".into());
    }
    let root = wide(
        &p.components()
            .take(2)
            .collect::<PathBuf>()
            .to_string_lossy(),
    );
    let kind = unsafe { GetDriveTypeW(root.as_ptr()) };
    if ![DRIVE_FIXED, DRIVE_REMOVABLE].contains(&kind) {
        return Err("LIBRARY_PATH_UNSAFE".into());
    }
    Ok(p)
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}
pub(crate) fn open_folder(s: &str) -> Result<(), String> {
    crate::portable_store::no_links(&local_path(s)?).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
    if !std::path::Path::new(s).is_dir() {
        return Err("LIBRARY_OFFLINE".into());
    }
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            wide("open").as_ptr(),
            wide(s).as_ptr(),
            null(),
            null(),
            1,
        )
    };
    if result as isize <= 32 {
        return Err("LIBRARY_OPEN_FAILED".into());
    }
    Ok(())
}
pub fn folder(args: &[String]) -> Result<i32, String> {
    if args.len() != 1 {
        return Err("Expected --library-folder choose|open".into());
    }
    if ["open", "validate"].contains(&args[0].as_str()) {
        let mut b = Vec::new();
        std::io::stdin()
            .take(32769)
            .read_to_end(&mut b)
            .map_err(|e| e.to_string())?;
        if b.len() > 32768 {
            return Err("LIBRARY_PATH_UNSAFE".into());
        }
        let v: serde_json::Value = serde_json::from_slice(&b).map_err(|_| "LIBRARY_PATH_UNSAFE")?;
        let s = v["path"].as_str().ok_or("LIBRARY_PATH_UNSAFE")?;
        local_path(s)?;
        if args[0] == "validate" {
            println!("{}", serde_json::json!({"valid":true}));
            return Ok(0);
        }
        open_folder(s)?;
        println!("{}", serde_json::json!({"opened":true}));
        return Ok(0);
    }
    if args[0] != "choose" {
        return Err("Unknown library folder action".into());
    }
    let result = match choose_folder()? {
        Some(path) => serde_json::json!({"cancelled":false,"path":path}),
        None => serde_json::json!({"cancelled":true}),
    };
    println!("{result}");
    Ok(0)
}
pub(crate) fn choose_folder() -> Result<Option<PathBuf>, String> {
    unsafe {
        let hr = CoInitializeEx(
            null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        );
        if hr < 0 {
            return Err("LIBRARY_DIALOG_UNAVAILABLE".into());
        }
        let title =
            wide("選擇 Atmos 桌布素材庫的位置 / Choose a parent folder for the Atmos library");
        let mut display = vec![0u16; 32768];
        let info = BROWSEINFOW {
            hwndOwner: null_mut(),
            pidlRoot: null_mut(),
            pszDisplayName: display.as_mut_ptr(),
            lpszTitle: title.as_ptr(),
            ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
            lpfn: None,
            lParam: 0,
            iImage: 0,
        };
        let item = SHBrowseForFolderW(&info);
        if item.is_null() {
            CoUninitialize();
            return Ok(None);
        }
        let mut text = vec![0u16; 32768];
        let ok = SHGetPathFromIDListEx(item, text.as_mut_ptr(), text.len() as u32, 0);
        CoTaskMemFree(item as _);
        CoUninitialize();
        if ok == 0 {
            return Err("LIBRARY_PATH_UNSAFE".into());
        }
        let s = String::from_utf16_lossy(
            &text[..text.iter().position(|c| *c == 0).unwrap_or(text.len())],
        );
        Ok(Some(local_path(&s)?))
    }
}
#[cfg(not(feature = "native-core"))]
pub fn command(args: Vec<String>) -> Result<i32, String> {
    use std::{
        io::Write,
        os::windows::process::CommandExt,
        process::{Command, Stdio},
    };
    let root = crate::installed::payload_root()?;
    crate::installed::verify(&root)?;
    let mut input = Vec::new();
    if args
        .get(1)
        .is_some_and(|s| matches!(s.as_str(), "move" | "reset" | "result" | "archive"))
    {
        std::io::stdin()
            .take(32769)
            .read_to_end(&mut input)
            .map_err(|e| e.to_string())?;
    }
    if input.len() > 32768 {
        return Err("LIBRARY_COMMAND_INVALID".into());
    }
    let mut c = Command::new(root.join("runtime/node/node.exe"));
    c.arg(root.join("app/implementation-p4_2_5/src/library-cli.mjs"))
        .args(args.into_iter().skip(1))
        .env(
            "ATMOS_DESKTOP_HOST_EXE",
            std::env::current_exe().map_err(|e| e.to_string())?,
        )
        .env(
            "ATMOS_LIBRARY_INSTALL_ROOT",
            std::env::current_exe()
                .map_err(|e| e.to_string())?
                .parent()
                .ok_or("Missing installation directory")?,
        )
        .env_remove("NODE_OPTIONS")
        .env_remove("NODE_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000);
    let mut child = c.spawn().map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&input)
        .map_err(|e| e.to_string())?;
    let result = child.wait_with_output().map_err(|e| e.to_string())?;
    if result.stdout.len() > 1048576 || result.stdout.is_empty() {
        return Err("LIBRARY_RESPONSE_INVALID".into());
    }
    std::io::stdout()
        .write_all(&result.stdout)
        .map_err(|e| e.to_string())?;
    Ok(result.status.code().unwrap_or(1))
}
#[cfg(test)]
mod tests {
    #[test]
    fn folder_paths_are_local_and_bounded() {
        assert!(super::local_path(r"C:\合法 資料").is_ok());
        assert!(super::local_path(r"\\server\share").is_err());
        assert!(super::local_path("relative").is_err());
        assert!(super::local_path("C:\\a\n").is_err());
    }
}
