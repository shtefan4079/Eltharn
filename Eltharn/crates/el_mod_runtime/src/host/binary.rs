use super::{Command, Host, ec};
use crate::{binary, lifecycle::Phase, wire::strict_json};
use el_extension_api::abi::ErrorCode;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
enum Request {
    Read(Read),
    Change(binary::Mutation),
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Read {
    Utf8Read {
        handle: String,
        offset: usize,
        count: usize,
    },
    ScalarRead {
        handle: String,
        offset: usize,
        scalar: binary::Scalar,
        endian: binary::Endian,
    },
    Metadata {
        handle: String,
    },
    Page {
        handle: String,
        offset: usize,
        count: usize,
    },
}
impl Host {
    pub(super) fn do_binary(&mut self, request: &[u8], response: &mut [u8]) -> i32 {
        if !self.has(binary::CAPABILITY) {
            return ec(ErrorCode::CapabilityDenied);
        }
        if !matches!(
            self.phase_of(),
            Ok(Phase::Init
                | Phase::AppTick
                | Phase::WorldEnter
                | Phase::Tick
                | Phase::WorldExit
                | Phase::Shutdown)
        ) {
            return ec(ErrorCode::WrongPhase);
        }
        let request = match strict_json(request, 4096)
            .and_then(|v| serde_json::from_value::<Request>(v).map_err(|e| e.to_string()))
        {
            Ok(request) => request,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let Some(store) = self.snapshot.binary.as_ref() else {
            return ec(ErrorCode::Unavailable);
        };
        let mut pending = None;
        let result = match request {
            Request::Read(Read::Utf8Read {
                handle,
                offset,
                count,
            }) => store.bytes(&handle).and_then(|data| {
                let end = offset
                    .checked_add(count)
                    .ok_or("Binary text range overflow")?;
                let bytes = data
                    .get(offset..end)
                    .ok_or("Binary text range outside buffer")?;
                let text = std::str::from_utf8(bytes).map_err(|_| "Invalid UTF-8 bytes")?;
                Ok(serde_json::json!(text))
            }),
            Request::Read(Read::ScalarRead {
                handle,
                offset,
                scalar,
                endian,
            }) => store
                .page(&handle, offset, scalar.width())
                .and_then(|data| scalar.decode(&data, endian)),
            Request::Read(Read::Metadata { handle }) => store
                .metadata(&handle)
                .and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string())),
            Request::Read(Read::Page {
                handle,
                offset,
                count,
            }) => store
                .page(&handle, offset, count)
                .and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string())),
            Request::Change(operation) => {
                let mut candidate = store.clone();
                let work = match candidate.work(&operation) {
                    Ok(w) => w,
                    Err(_) => return ec(ErrorCode::InvalidRequest),
                };
                if self.binary_work.saturating_add(work) > binary::MAX_WORK {
                    return ec(ErrorCode::LimitExceeded);
                }
                let result = candidate
                    .apply(&operation)
                    .and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string()));
                pending = Some((operation, candidate, work));
                result
            }
        };
        let bytes = match result.and_then(|v| serde_json::to_vec(&v).map_err(|e| e.to_string())) {
            Ok(b) => b,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if bytes.len() > response.len() {
            return ec(ErrorCode::BufferTooSmall);
        }
        if let Some((operation, candidate, work)) = pending {
            if let Err(error) = self.stage_cmd(Command::Binary { operation }) {
                return ec(error);
            }
            self.snapshot.binary = Some(candidate);
            self.binary_work += work;
        }
        response[..bytes.len()].copy_from_slice(&bytes);
        bytes.len() as i32
    }
}
