use super::{Command, Host, ec};
use crate::{
    config::{self, Change},
    wire::strict_json,
};
use el_extension_api::{abi::ErrorCode, services};
use serde_json::{Value, json};
impl Host {
    pub(super) fn do_config(&mut self, op: i32, req: &[u8], res: &mut [u8]) -> i32 {
        let permitted = match op {
            services::CONFIG_READ_OPCODE => self.has(config::READ_CAP),
            services::CONFIG_WRITE_OPCODE | services::CONFIG_DELETE_OPCODE => {
                self.has(config::WRITE_CAP)
            }
            services::CONFIG_STATUS_OPCODE => {
                self.has(config::READ_CAP) || self.has(config::WRITE_CAP)
            }
            _ => false,
        };
        if !permitted {
            return ec(ErrorCode::CapabilityDenied);
        }
        let Some(snapshot) = self.snapshot.config.as_ref() else {
            return ec(ErrorCode::InvalidRequest);
        };
        let output = if op == services::CONFIG_STATUS_OPCODE {
            if !req.is_empty() {
                return ec(ErrorCode::InvalidRequest);
            }
            json!({"revision":snapshot.record.revision,"last_write":snapshot.status})
        } else {
            let request = match strict_json(req, config::MAX_VALUE_BYTES + 256) {
                Ok(Value::Object(m)) => m,
                _ => return ec(ErrorCode::InvalidRequest),
            };
            let Some(key) = request.get("key").and_then(Value::as_str) else {
                return ec(ErrorCode::InvalidRequest);
            };
            if config::key(key).is_err()
                || request.len()
                    != if op == services::CONFIG_WRITE_OPCODE {
                        2
                    } else {
                        1
                    }
            {
                return ec(ErrorCode::InvalidRequest);
            }
            if op == services::CONFIG_READ_OPCODE {
                snapshot
                    .record
                    .values
                    .get(key)
                    .cloned()
                    .unwrap_or(Value::Null)
            } else {
                let change = if op == services::CONFIG_WRITE_OPCODE {
                    let Some(value) = request.get("value") else {
                        return ec(ErrorCode::InvalidRequest);
                    };
                    if config::value(value).is_err() {
                        return ec(ErrorCode::LimitExceeded);
                    }
                    Change::Set {
                        key: key.into(),
                        value: value.clone(),
                    }
                } else {
                    Change::Delete { key: key.into() }
                };
                return match self.stage_cmd(Command::Config { change }) {
                    Ok(()) => 0,
                    Err(error) => ec(error),
                };
            }
        };
        match serde_json::to_vec(&output) {
            Ok(bytes) if bytes.len() <= res.len() => {
                res[..bytes.len()].copy_from_slice(&bytes);
                bytes.len() as i32
            }
            _ => ec(ErrorCode::BufferTooSmall),
        }
    }
}
