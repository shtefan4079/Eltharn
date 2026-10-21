use el_protocol_backend::{
    BackendNetworkEvent, BackendToClientMessage, ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
};
pub fn into_event(
    message: BackendToClientMessage,
    requested_version: &str,
) -> Result<Option<BackendNetworkEvent>, String> {
    match message {
        BackendToClientMessage::Hello { manifest } => {
            if manifest.ipc_version != ELTHARN_PROTOCOL_BACKEND_IPC_VERSION {
                return Err(format!(
                    "Eltharn protocol backend IPC mismatch: backend={} client={}",
                    manifest.ipc_version, ELTHARN_PROTOCOL_BACKEND_IPC_VERSION
                ));
            }
            if manifest.minecraft_version != requested_version {
                return Err(format!(
                    "Eltharn protocol backend version mismatch: backend targets Minecraft {}, but client requested {}",
                    manifest.minecraft_version, requested_version
                ));
            }
            Ok(None)
        }
        BackendToClientMessage::Event(event) => Ok(Some(event)),
        BackendToClientMessage::Status { .. } => Ok(None),
        BackendToClientMessage::Disconnected { reason } => Err(reason),
    }
}
