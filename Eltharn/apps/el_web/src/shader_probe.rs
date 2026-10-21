use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub async fn shader_probe_configure(bytes: &[u8]) -> Result<(), JsValue> {
    el_client::shaderpacks::configure_packet(bytes)
        .await
        .map_err(|e| JsValue::from_str(&e))
}
#[wasm_bindgen]
pub fn shader_probe_report() -> String {
    el_client::shaderpacks::report().to_string()
}
#[wasm_bindgen]
pub fn shader_probe_disable() {
    el_client::shaderpacks::request_disable();
}
