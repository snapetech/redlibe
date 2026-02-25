#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
use std::fs;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use rand::RngCore;
use tauri::{Manager, RunEvent};

struct AppState {
	child: Mutex<Option<Child>>,
}

fn main() {
	let state = AppState { child: Mutex::new(None) };

	tauri::Builder::default()
		.setup(|app| {
			let cfg_dir = desktop_config_dir().ok_or("Could not determine desktop config dir")?;
			fs::create_dir_all(&cfg_dir)?;
			let secret = ensure_session_secret(&cfg_dir)?;
			ensure_desktop_config_file(&cfg_dir)?;
			let port = choose_port(18080, 25)?;
			let url = format!("http://127.0.0.1:{port}/");
			let child = spawn_backend(&cfg_dir, &secret, port)?;
			wait_for_server(port, Duration::from_secs(15)).map_err(io::Error::other)?;

			{
				let app_state: tauri::State<'_, AppState> = app.state();
				*app_state.child.lock().map_err(|_| io::Error::other("poisoned mutex"))? = Some(child);
			}

			if let Some(window) = app.get_webview_window("main") {
				let _ = window.navigate(url.parse()?);
			}

			Ok(())
		})
		.manage(state)
		.run(move |app, event| {
			if let RunEvent::ExitRequested { .. } = event {
				let state: tauri::State<'_, AppState> = app.state();
				if let Ok(mut guard) = state.child.lock() {
					if let Some(mut child) = guard.take() {
						let _ = child.kill();
					}
				}
			}
		})
		.expect("error while running tauri application");
}

fn spawn_backend(cfg_dir: &Path, session_secret: &str, port: u16) -> io::Result<Child> {
	let server_bin = find_server_binary();
	Command::new(server_bin)
		.arg("--address")
		.arg("127.0.0.1")
		.arg("--port")
		.arg(port.to_string())
		.current_dir(cfg_dir)
		.env("REDLIB_SESSION_SECRET", session_secret)
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
}

fn find_server_binary() -> PathBuf {
	if let Some(path) = env::var_os("REDLIB_SERVER_BIN") {
		return PathBuf::from(path);
	}
	let exe_dir = env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
	if let Some(dir) = exe_dir {
		let sibling = dir.join(server_exe_name());
		if sibling.is_file() {
			return sibling;
		}
	}
	PathBuf::from(server_exe_name())
}

fn server_exe_name() -> &'static str {
	if cfg!(target_os = "windows") {
		"redlib.exe"
	} else {
		"redlib"
	}
}

fn wait_for_server(port: u16, timeout: Duration) -> Result<(), String> {
	let deadline = Instant::now() + timeout;
	while Instant::now() < deadline {
		if TcpStream::connect(("127.0.0.1", port)).is_ok() {
			return Ok(());
		}
		thread::sleep(Duration::from_millis(150));
	}
	Err(format!("timed out waiting for 127.0.0.1:{port}"))
}

fn choose_port(start: u16, attempts: u16) -> Result<u16, io::Error> {
	for offset in 0..attempts {
		let port = start + offset;
		if TcpListener::bind(("127.0.0.1", port)).is_ok() {
			return Ok(port);
		}
	}
	Err(io::Error::new(io::ErrorKind::AddrInUse, "no free local port"))
}

fn ensure_session_secret(cfg_dir: &Path) -> io::Result<String> {
	let path = cfg_dir.join("session_secret");
	if let Ok(existing) = fs::read_to_string(&path) {
		let trimmed = existing.trim().to_string();
		if trimmed.len() >= 32 {
			return Ok(trimmed);
		}
	}
	let mut bytes = [0u8; 32];
	rand::rngs::OsRng.fill_bytes(&mut bytes);
	let secret = hex_encode(&bytes);
	fs::write(path, format!("{secret}\n"))?;
	Ok(secret)
}

fn ensure_desktop_config_file(cfg_dir: &Path) -> io::Result<()> {
	let path = cfg_dir.join("redlib.toml");
	if path.exists() {
		return Ok(());
	}
	fs::write(path, "REDLIB_SECURE_COOKIES = \"false\"\n")
}

fn desktop_config_dir() -> Option<PathBuf> {
	let app_dir = "redlibe";
	if cfg!(target_os = "windows") {
		return env::var_os("APPDATA").map(|p| PathBuf::from(p).join(app_dir));
	}
	if cfg!(target_os = "macos") {
		return env::var_os("HOME")
			.map(|h| PathBuf::from(h).join("Library").join("Application Support").join(app_dir));
	}
	if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
		return Some(PathBuf::from(xdg).join(app_dir));
	}
	env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join(app_dir))
}

fn hex_encode(bytes: &[u8]) -> String {
	const HEX: &[u8; 16] = b"0123456789abcdef";
	let mut out = String::with_capacity(bytes.len() * 2);
	for b in bytes {
		out.push(HEX[(b >> 4) as usize] as char);
		out.push(HEX[(b & 0x0f) as usize] as char);
	}
	out
}
