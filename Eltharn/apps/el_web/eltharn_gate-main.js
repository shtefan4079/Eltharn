import init, * as wasmExports from "./eltharn_gate.js";
import { mountExtensions } from "./extensions.js";

console.log("[COI Check] crossOriginIsolated =", window.crossOriginIsolated);
console.log("[COI Check] SharedArrayBuffer available =", typeof SharedArrayBuffer !== "undefined");

console.log("[GPU Diagnostic] checking graphics API availability...");
console.log("[GPU Diagnostic] navigator.gpu:", Boolean(navigator.gpu));
if (navigator.gpu) {
  navigator.gpu.requestAdapter().then(adapter => {
    console.log("[GPU Diagnostic] WebGPU adapter result:", adapter ? adapter : "null (WebGPU disabled/blocklisted)");
  }).catch(err => console.warn("[GPU Diagnostic] WebGPU requestAdapter error:", err));
}

const testCanvas = document.createElement("canvas");
const gl2 = testCanvas.getContext("webgl2");
console.log("[GPU Diagnostic] WebGL2 context:", Boolean(gl2));
if (gl2) {
  const debugInfo = gl2.getExtension("WEBGL_debug_renderer_info");
  if (debugInfo) {
    console.log("[GPU Diagnostic] WebGL2 Vendor:", gl2.getParameter(debugInfo.UNMASKED_VENDOR_WEBGL));
    console.log("[GPU Diagnostic] WebGL2 Renderer:", gl2.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL));
  }
} else {
  const gl1 = testCanvas.getContext("webgl");
  console.log("[GPU Diagnostic] WebGL1 fallback context:", Boolean(gl1));
}

init().then(async () => {
  if (wasmExports.extension_default_preferences) await mountExtensions(wasmExports);
}).catch((error) => {
  console.error("Eltharn Gate WASM failed to start", error);
  const failure = document.getElementById("eltharn_gate-error");
  if (failure) {
    failure.textContent = "Failed to start WASM: " + error;
    failure.dataset.tone = "error";
    failure.removeAttribute("hidden");
  }
});
