#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rand::RngCore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cfg_dir = desktop_config_dir().ok_or("Could not determine desktop config directory")?;
	fs::create_dir_all(&cfg_dir)?;

	let session_secret = ensure_session_secret(&cfg_dir)?;
	ensure_desktop_config_file(&cfg_dir)?;

	let port = choose_port(18080, 25)?;
	let url = format!("http://127.0.0.1:{port}/");
	let mut child = spawn_server(&cfg_dir, &session_secret, port)?;

	match wait_for_server(port, Duration::from_secs(15)) {
		Ok(()) => {
			let _ = open_url(&url);
		}
		Err(e) => {
			let _ = child.kill();
			return Err(Box::<dyn std::error::Error>::from(format!("Server failed to start: {e}")));
		}
	}

	let status = child.wait()?;
	if !status.success() {
		return Err(Box::<dyn std::error::Error>::from(format!("redlib exited with status {status}")));
	}
	Ok(())
}

fn spawn_server(cfg_dir: &Path, session_secret: &str, port: u16) -> Result<Child, Box<dyn std::error::Error>> {
	let server_bin = find_server_binary();
	let mut cmd = Command::new(server_bin);
	cmd.arg("--address")
		.arg("127.0.0.1")
		.arg("--port")
		.arg(port.to_string())
		.current_dir(cfg_dir)
		.env("REDLIB_SESSION_SECRET", session_secret)
		.env("REDLIB_DESKTOP_MODE", "1")
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit());

	Ok(cmd.spawn()?)
}

fn find_server_binary() -> PathBuf {
	if let Some(p) = env::var_os("REDLIB_SERVER_BIN") {
		return PathBuf::from(p);
	}

	if let Ok(current) = env::current_exe() {
		if let Some(dir) = current.parent() {
			let sibling = dir.join(server_exe_name());
			if sibling.is_file() {
				return sibling;
			}
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
	let addr: SocketAddr = format!("127.0.0.1:{port}")
		.parse()
		.map_err(|e| format!("Invalid local socket address: {e}"))?;
	let deadline = Instant::now() + timeout;
	while Instant::now() < deadline {
		if TcpStream::connect_timeout(&addr, Duration::from_millis(250)).is_ok() {
			return Ok(());
		}
		thread::sleep(Duration::from_millis(150));
	}
	Err(format!("timed out waiting for 127.0.0.1:{port}"))
}

fn choose_port(start: u16, attempts: u16) -> Result<u16, String> {
	for offset in 0..attempts {
		let port = start.saturating_add(offset);
		if TcpListener::bind(("127.0.0.1", port)).is_ok() {
			return Ok(port);
		}
	}
	Err(format!("No free local port found in range {start}-{}", start.saturating_add(attempts)))
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
	fs::write(&path, format!("{secret}\n"))?;
	Ok(secret)
}

fn ensure_desktop_config_file(cfg_dir: &Path) -> io::Result<()> {
	let path = cfg_dir.join("redlib.toml");
	if path.exists() {
		return Ok(());
	}
	let contents = r#"# redlib desktop launcher config
# This file is read because the launcher starts the backend with cwd set here.

REDLIB_SECURE_COOKIES = "false"
"#;
	fs::write(path, contents)
}

fn desktop_config_dir() -> Option<PathBuf> {
	let app_dir = "redlibe";

	if cfg!(target_os = "windows") {
		let base = env::var_os("APPDATA")?;
		return Some(PathBuf::from(base).join(app_dir));
	}

	if cfg!(target_os = "macos") {
		let home = env::var_os("HOME")?;
		return Some(PathBuf::from(home).join("Library").join("Application Support").join(app_dir));
	}

	if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
		return Some(PathBuf::from(xdg).join(app_dir));
	}
	let home = env::var_os("HOME")?;
	Some(PathBuf::from(home).join(".config").join(app_dir))
}

fn open_url(url: &str) -> io::Result<()> {
	#[cfg(target_os = "windows")]
	{
		Command::new("cmd").args(["/C", "start", "", url]).spawn()?.wait()?;
		return Ok(());
	}
	#[cfg(target_os = "macos")]
	{
		Command::new("open").arg(url).spawn()?.wait()?;
		return Ok(());
	}
	#[cfg(not(any(target_os = "windows", target_os = "macos")))]
	{
		Command::new("xdg-open").arg(url).spawn()?.wait()?;
		Ok(())
	}
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
