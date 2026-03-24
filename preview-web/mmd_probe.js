const out = document.getElementById("probe");

async function run() {
  const stateRes = await fetch("/state");
  if (!stateRes.ok) {
    throw new Error("failed to fetch /state");
  }
  const state = await stateRes.json();

  const lines = [
    `glb: ${state.glb_name}`,
    `camera_vmd: ${state.camera_vmd_name || "none"}`,
    `sync_offset_ms: ${state.sync_offset_ms ?? 0}`,
    `sync_profile_key: ${state.sync_profile_key || "none"}`,
    `sync_profile_hit: ${state.sync_profile_hit === true}`,
    "",
    "Web import paths",
    "- GLTFLoader: native GLB import",
    "- MMDLoader: PMX + VMD bridge for parity checks",
    "- Runtime scope: PMX/VMD direct parse is not enabled",
  ];
  out.textContent = lines.join("\n");
}

run().catch((err) => {
  out.textContent = String(err);
  console.error(err);
});
