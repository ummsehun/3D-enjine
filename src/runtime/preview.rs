use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};

use crate::runtime::cli::PreviewArgs;

const EMBED_INDEX_HTML: &str = r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <title>Terminal Miku 3D Preview</title>
  <style>
    html,body,#app{margin:0;padding:0;width:100%;height:100%;background:#111827;color:#d1d5db;font-family:ui-sans-serif,system-ui}
    #hud{position:fixed;left:12px;top:12px;background:rgba(0,0,0,.45);padding:8px 10px;border-radius:8px;font-size:12px;line-height:1.4}
    #err{position:fixed;left:12px;bottom:12px;color:#fecaca}
  </style>
</head>
<body>
  <div id="app"></div>
  <div id="hud">loading...</div>
  <div id="err"></div>
  <script type="module" src="/app.js"></script>
</body>
</html>
"#;

const EMBED_APP_JS: &str = r#"import * as THREE from 'https://unpkg.com/three@0.170.0/build/three.module.js';
import { GLTFLoader } from 'https://unpkg.com/three@0.170.0/examples/jsm/loaders/GLTFLoader.js';

const app = document.getElementById('app');
const hud = document.getElementById('hud');
const err = document.getElementById('err');
const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
renderer.setSize(window.innerWidth, window.innerHeight);
app.appendChild(renderer.domElement);
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x111827);
const camera = new THREE.PerspectiveCamera(55, window.innerWidth / window.innerHeight, 0.01, 400);
camera.position.set(0, 1.2, 3.0);
const light = new THREE.DirectionalLight(0xffffff, 1.2);
light.position.set(3, 4, 2);
scene.add(new THREE.AmbientLight(0xffffff, 0.6), light);
const clock = new THREE.Clock();
let mixer = null;
let actions = [];
let state = null;

window.addEventListener('resize', () => {
  camera.aspect = window.innerWidth / window.innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(window.innerWidth, window.innerHeight);
});

async function loadState() {
  const res = await fetch('/state');
  if (!res.ok) throw new Error('state fetch failed');
  return await res.json();
}

function playAnimation(indexOrName) {
  if (!mixer || actions.length === 0) return;
  let action = actions[0];
  if (typeof indexOrName === 'number' && indexOrName >= 0 && indexOrName < actions.length) {
    action = actions[indexOrName];
  } else if (typeof indexOrName === 'string') {
    const found = actions.find(a => a.getClip().name === indexOrName);
    if (found) action = found;
  }
  actions.forEach(a => a.stop());
  action.reset().play();
}

async function init() {
  state = await loadState();
  const loader = new GLTFLoader();
  const gltf = await loader.loadAsync(state.glb_url);
  scene.add(gltf.scene);
  if (gltf.animations && gltf.animations.length > 0) {
    mixer = new THREE.AnimationMixer(gltf.scene);
    actions = gltf.animations.map(clip => mixer.clipAction(clip));
    playAnimation(state.anim_selector ?? 0);
  }
  hud.textContent = `preview | mode=${state.camera_mode} | glb=${state.glb_name}`;
}

let sync = { master_sec: 0, speed_factor: 1.0, sync_offset_ms: 0, playing: true, seq: 0 };
let localClockSec = 0;
let lastSyncAt = performance.now();
let ws = null;

function applySync(data) {
  const master = data.master_sec + (data.sync_offset_ms || 0) / 1000.0;
  const errSec = master - localClockSec;
  if (Math.abs(errSec) > 0.12) {
    localClockSec = master;
  } else {
    localClockSec += errSec * 0.15;
  }
  sync = data;
  lastSyncAt = performance.now();
}

function connectSyncSocket() {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const url = `${proto}://${location.host}/sync`;
  ws = new WebSocket(url);
  ws.onopen = () => {
    lastSyncAt = performance.now();
  };
  ws.onmessage = (ev) => {
    try {
      applySync(JSON.parse(ev.data));
    } catch (_) {}
  };
  ws.onerror = () => {};
  ws.onclose = () => {
    setTimeout(connectSyncSocket, 1200);
  };
}

async function fallbackPoll() {
  if (ws && ws.readyState === WebSocket.OPEN) return;
  try {
    const res = await fetch('/sync');
    if (!res.ok) return;
    const data = await res.json();
    applySync(data);
  } catch (_) {}
}
setInterval(fallbackPoll, 250);

function tick() {
  requestAnimationFrame(tick);
  const dt = clock.getDelta();
  const speed = Number.isFinite(sync.speed_factor) ? sync.speed_factor : 1.0;
  if (sync.playing !== false) {
    localClockSec += dt * speed;
  }
  if (mixer) {
    mixer.update(dt * speed);
  }
  const staleMs = performance.now() - lastSyncAt;
  hud.textContent = `preview | mode=${state?.camera_mode ?? 'n/a'} | t=${localClockSec.toFixed(3)} | sync_seq=${sync.seq} | stale=${staleMs.toFixed(0)}ms`;
  renderer.render(scene, camera);
}

init().then(() => {
  connectSyncSocket();
  tick();
}).catch((e) => { err.textContent = String(e); console.error(e); });
"#;

#[derive(Debug)]
struct PreviewState {
    glb_path: PathBuf,
    camera_vmd_path: Option<PathBuf>,
    anim_selector: Option<String>,
    camera_mode: String,
    speed_factor: f32,
    sync_offset_ms: i32,
}

#[derive(Debug)]
struct PreviewRuntime {
    state: PreviewState,
    started: Instant,
    seq: AtomicU64,
}

impl PreviewRuntime {
    fn sync_payload(&self) -> String {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed).saturating_add(1);
        let master_sec = self.started.elapsed().as_secs_f32();
        format!(
            "{{\"master_sec\":{:.6},\"speed_factor\":{:.6},\"sync_offset_ms\":{},\"playing\":true,\"seq\":{}}}",
            master_sec, self.state.speed_factor, self.state.sync_offset_ms, seq
        )
    }
}

pub fn run_preview_server(args: &PreviewArgs, camera_vmd_path: Option<PathBuf>) -> Result<()> {
    if !args.glb.exists() {
        bail!("preview GLB not found: {}", args.glb.display());
    }
    let listener = TcpListener::bind(("127.0.0.1", args.port))
        .with_context(|| format!("failed to bind preview server on 127.0.0.1:{}", args.port))?;
    let runtime = Arc::new(PreviewRuntime {
        state: PreviewState {
            glb_path: args.glb.clone(),
            camera_vmd_path,
            anim_selector: args.anim.clone(),
            camera_mode: format!("{:?}", args.camera_mode).to_lowercase(),
            speed_factor: 1.0,
            sync_offset_ms: 0,
        },
        started: Instant::now(),
        seq: AtomicU64::new(0),
    });
    println!(
        "preview server running: http://127.0.0.1:{}/  (Ctrl+C to stop)",
        args.port
    );

    for conn in listener.incoming() {
        let stream = match conn {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        let runtime = runtime.clone();
        thread::spawn(move || {
            let _ = handle_connection(stream, runtime);
        });
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, runtime: Arc<PreviewRuntime>) -> Result<()> {
    let request = read_http_request(&mut stream)?;
    if request.path == "/sync" && request.is_websocket_upgrade() {
        websocket_handshake(&mut stream, &request)?;
        websocket_sync_loop(stream, runtime)?;
        return Ok(());
    }

    match request.path.as_str() {
        "/" | "/index.html" => {
            let body =
                load_preview_file("index.html").unwrap_or_else(|| EMBED_INDEX_HTML.to_owned());
            write_http_text(&mut stream, 200, "text/html; charset=utf-8", &body)?;
        }
        "/app.js" => {
            let body = load_preview_file("app.js").unwrap_or_else(|| EMBED_APP_JS.to_owned());
            write_http_text(
                &mut stream,
                200,
                "application/javascript; charset=utf-8",
                &body,
            )?;
        }
        "/state" => {
            let glb_name = runtime
                .state
                .glb_path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("model.glb");
            let camera_name = runtime
                .state
                .camera_vmd_path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|v| v.to_str())
                .unwrap_or("");
            let anim_selector = runtime.state.anim_selector.clone().unwrap_or_default();
            let body = format!(
                "{{\"glb_url\":\"/asset/glb\",\"glb_name\":\"{}\",\"camera_vmd_url\":\"/asset/camera\",\"camera_vmd_name\":\"{}\",\"anim_selector\":\"{}\",\"camera_mode\":\"{}\"}}",
                json_escape(glb_name),
                json_escape(camera_name),
                json_escape(&anim_selector),
                json_escape(&runtime.state.camera_mode),
            );
            write_http_text(&mut stream, 200, "application/json", &body)?;
        }
        "/sync" => {
            // HTTP fallback path for environments that cannot open WebSocket.
            let body = runtime.sync_payload();
            write_http_text(&mut stream, 200, "application/json", &body)?;
        }
        "/asset/glb" => {
            write_http_file(&mut stream, &runtime.state.glb_path, "model/gltf-binary")?;
        }
        "/asset/camera" => {
            if let Some(path) = runtime.state.camera_vmd_path.as_ref() {
                write_http_file(&mut stream, path, "application/octet-stream")?;
            } else {
                write_http_text(
                    &mut stream,
                    404,
                    "text/plain; charset=utf-8",
                    "no camera vmd",
                )?;
            }
        }
        _ => {
            write_http_text(&mut stream, 404, "text/plain; charset=utf-8", "not found")?;
        }
    }
    Ok(())
}

fn load_preview_file(name: &str) -> Option<String> {
    let path = PathBuf::from("preview-web").join(name);
    fs::read_to_string(path).ok()
}

#[derive(Debug)]
struct HttpRequest {
    path: String,
    headers: HashMap<String, String>,
}

impl HttpRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    fn is_websocket_upgrade(&self) -> bool {
        let is_upgrade = self
            .header("upgrade")
            .map(|v| v.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false);
        let has_connection_upgrade = self
            .header("connection")
            .map(|v| {
                v.split(',')
                    .any(|part| part.trim().eq_ignore_ascii_case("upgrade"))
            })
            .unwrap_or(false);
        let has_key = self.header("sec-websocket-key").is_some();
        is_upgrade && has_connection_upgrade && has_key
    }
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let mut buf = [0_u8; 8192];
    let size = stream.read(&mut buf).context("read request")?;
    if size == 0 {
        bail!("empty request");
    }
    let request = String::from_utf8_lossy(&buf[..size]);
    let mut lines = request.lines();
    let line = lines.next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_path = parts.next().unwrap_or("/");
    if method != "GET" {
        bail!("unsupported method: {method}");
    }
    let path = raw_path.split('?').next().unwrap_or("/").to_owned();
    let mut headers = HashMap::new();
    for raw_line in lines {
        let line = raw_line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    Ok(HttpRequest { path, headers })
}

fn write_http_text(
    stream: &mut TcpStream,
    code: u16,
    content_type: &str,
    body: &str,
) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        code,
        content_type,
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .context("write response")
}

fn write_http_file(stream: &mut TcpStream, path: &PathBuf, content_type: &str) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("read file: {}", path.display()))?;
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        content_type,
        bytes.len()
    );
    stream.write_all(head.as_bytes()).context("write head")?;
    stream.write_all(&bytes).context("write body")
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn websocket_handshake(stream: &mut TcpStream, request: &HttpRequest) -> Result<()> {
    let key = request
        .header("sec-websocket-key")
        .context("missing sec-websocket-key")?;
    let accept = websocket_accept_key(key);
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accept
    );
    stream
        .write_all(response.as_bytes())
        .context("write websocket handshake")
}

fn websocket_sync_loop(mut stream: TcpStream, runtime: Arc<PreviewRuntime>) -> Result<()> {
    stream
        .set_nodelay(true)
        .context("set_nodelay for websocket")?;
    loop {
        let payload = runtime.sync_payload();
        if write_ws_text_frame(&mut stream, payload.as_bytes()).is_err() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

fn write_ws_text_frame(stream: &mut TcpStream, payload: &[u8]) -> Result<()> {
    let mut frame = Vec::with_capacity(payload.len().saturating_add(10));
    frame.push(0x81); // FIN + text
    if payload.len() < 126 {
        frame.push(payload.len() as u8);
    } else if payload.len() <= 0xFFFF {
        frame.push(126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    frame.extend_from_slice(payload);
    stream.write_all(&frame).context("write websocket frame")
}

fn websocket_accept_key(key: &str) -> String {
    let mut input = Vec::with_capacity(key.len() + 36);
    input.extend_from_slice(key.trim().as_bytes());
    input.extend_from_slice(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64_encode(&sha1_digest(&input))
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut i = 0usize;
    while i + 3 <= data.len() {
        let chunk = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(TABLE[((chunk >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 0x3F) as usize] as char);
        out.push(TABLE[((chunk >> 6) & 0x3F) as usize] as char);
        out.push(TABLE[(chunk & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let chunk = (data[i] as u32) << 16;
        out.push(TABLE[((chunk >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let chunk = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
        out.push(TABLE[((chunk >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 0x3F) as usize] as char);
        out.push(TABLE[((chunk >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

fn sha1_digest(message: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let bit_len = (message.len() as u64) * 8;
    let mut padded = message.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0_u32; 80];
        for (i, block) in chunk.chunks_exact(4).take(16).enumerate() {
            w[i] = u32::from_be_bytes([block[0], block[1], block[2], block[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => (((b & c) | ((!b) & d)), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => (((b & c) | (b & d) | (c & d)), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0_u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_accept_matches_reference() {
        let accept = websocket_accept_key("dGhlIHNhbXBsZSBub25jZQ==");
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}
