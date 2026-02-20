import * as THREE from "https://unpkg.com/three@0.170.0/build/three.module.js";
import { GLTFLoader } from "https://unpkg.com/three@0.170.0/examples/jsm/loaders/GLTFLoader.js";

const app = document.getElementById("app");
const hud = document.getElementById("hud");
const err = document.getElementById("err");

const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
renderer.setSize(window.innerWidth, window.innerHeight);
app.appendChild(renderer.domElement);

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x111827);
const camera = new THREE.PerspectiveCamera(
  55,
  window.innerWidth / window.innerHeight,
  0.01,
  400,
);
camera.position.set(0, 1.2, 3.0);

const light = new THREE.DirectionalLight(0xffffff, 1.2);
light.position.set(3, 4, 2);
scene.add(new THREE.AmbientLight(0xffffff, 0.6), light);

window.addEventListener("resize", () => {
  camera.aspect = window.innerWidth / window.innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(window.innerWidth, window.innerHeight);
});

let mixer = null;
let actions = [];
let state = null;
let sync = { master_sec: 0, speed_factor: 1.0, sync_offset_ms: 0, playing: true, seq: 0 };
let localClockSec = 0;
let lastSyncAt = performance.now();
let ws = null;
const clock = new THREE.Clock();

function playAnimation(selector) {
  if (!mixer || actions.length === 0) return;
  let action = actions[0];
  if (typeof selector === "number" && selector >= 0 && selector < actions.length) {
    action = actions[selector];
  } else if (selector && selector.length > 0) {
    const named = actions.find((a) => a.getClip().name === selector);
    if (named) action = named;
  }
  actions.forEach((a) => a.stop());
  action.reset().play();
}

async function init() {
  const stateRes = await fetch("/state");
  if (!stateRes.ok) throw new Error("failed to fetch /state");
  state = await stateRes.json();

  const loader = new GLTFLoader();
  const gltf = await loader.loadAsync(state.glb_url);
  scene.add(gltf.scene);
  if (gltf.animations && gltf.animations.length > 0) {
    mixer = new THREE.AnimationMixer(gltf.scene);
    actions = gltf.animations.map((clip) => mixer.clipAction(clip));
    playAnimation(state.anim_selector);
  }
  hud.textContent = `preview | mode=${state.camera_mode} | glb=${state.glb_name}`;
}

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
  const proto = location.protocol === "https:" ? "wss" : "ws";
  ws = new WebSocket(`${proto}://${location.host}/sync`);
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
    const res = await fetch("/sync");
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
  hud.textContent = `preview | mode=${state?.camera_mode ?? "n/a"} | t=${localClockSec.toFixed(3)} | sync_seq=${sync.seq} | stale=${staleMs.toFixed(0)}ms`;
  renderer.render(scene, camera);
}

init()
  .then(() => {
    connectSyncSocket();
    tick();
  })
  .catch((e) => {
    err.textContent = String(e);
    console.error(e);
  });
