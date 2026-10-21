use crate::network::multiplayer::server_data::{ServerData, ServerType};
use el_nbt::io::{read_named_tag, write_named_tag};
use el_nbt::tag::{Tag, compound_get_tag};
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
pub struct ServerList {
    #[cfg(not(target_arch = "wasm32"))]
    pub game_directory: PathBuf,
    pub servers: Vec<ServerData>,
    pub hidden_servers: Vec<ServerData>,
}
#[allow(dead_code)]
impl ServerList {
    pub fn new(game_directory: PathBuf) -> Self {
        #[cfg(target_arch = "wasm32")]
        drop(game_directory);
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            game_directory,
            servers: Vec::new(),
            hidden_servers: Vec::new(),
        }
    }
    pub fn load(&mut self) {
        self.servers.clear();
        self.hidden_servers.clear();
        #[cfg(target_arch = "wasm32")]
        let tag_opt = {
            use base64::Engine;
            use std::io::Cursor;
            web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .and_then(|s| s.get_item("servers.dat").ok().flatten())
                .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
                .and_then(|bytes| {
                    let mut cursor = Cursor::new(bytes);
                    read_named_tag(&mut cursor).ok().map(|(_name, t)| t)
                })
        };
        #[cfg(not(target_arch = "wasm32"))]
        let tag_opt = {
            let path = self.game_directory.join("servers.dat");
            if !path.exists() {
                None
            } else if let Ok(file) = File::open(&path) {
                let mut reader = BufReader::new(file);
                if let Ok((_name, t)) = read_named_tag(&mut reader) {
                    Some(t)
                } else if let Ok(file2) = File::open(&path) {
                    el_nbt::io::read_gzip(file2).ok().map(|(_name, t)| t)
                } else {
                    None
                }
            } else {
                None
            }
        };
        let Some(tag) = tag_opt else {
            return;
        };
        if let Some(Tag::List(list)) = compound_get_tag(&tag, "servers") {
            for server_tag in list {
                let hidden = if let Some(Tag::Byte(b)) = compound_get_tag(server_tag, "hidden") {
                    *b != 0
                } else {
                    false
                };
                let server_data = ServerData::read(server_tag);
                if hidden {
                    self.hidden_servers.push(server_data);
                } else {
                    self.servers.push(server_data);
                }
            }
        }
    }
    pub fn save(&self) {
        let mut list_tags = Vec::new();
        for server in &self.servers {
            let mut server_tag = server.write();
            if let Tag::Compound(ref mut fields) = server_tag {
                fields.push(("hidden".to_string(), Tag::Byte(0)));
            }
            list_tags.push(server_tag);
        }
        for server in &self.hidden_servers {
            let mut server_tag = server.write();
            if let Tag::Compound(ref mut fields) = server_tag {
                fields.push(("hidden".to_string(), Tag::Byte(1)));
            }
            list_tags.push(server_tag);
        }
        let mut root_fields = Vec::new();
        root_fields.push(("servers".to_string(), Tag::List(list_tags)));
        let root_tag = Tag::Compound(root_fields);
        #[cfg(target_arch = "wasm32")]
        {
            use base64::Engine;
            use std::io::Cursor;
            let mut bytes = Vec::new();
            let mut cursor = Cursor::new(&mut bytes);
            if write_named_tag(&mut cursor, "", &root_tag).is_ok() {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                if let Some(storage) =
                    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
                {
                    let _ = storage.set_item("servers.dat", &b64);
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = self.game_directory.join("servers.dat");
            if let Ok(file) = File::create(&path) {
                let mut writer = BufWriter::new(file);
                let _ = write_named_tag(&mut writer, "", &root_tag);
            }
        }
    }
    pub fn get(&self, index: usize) -> Option<&ServerData> {
        self.servers.get(index)
    }
    pub fn get_by_ip(&self, ip: &str) -> Option<&ServerData> {
        for server in &self.servers {
            if server.ip == ip {
                return Some(server);
            }
        }
        for server in &self.hidden_servers {
            if server.ip == ip {
                return Some(server);
            }
        }
        None
    }
    pub fn get_or_add_hidden_other_server(
        &mut self,
        server_address: &str,
        default_name: impl FnOnce() -> String,
    ) -> ServerData {
        if let Some(server) = self.get_by_ip(server_address) {
            return server.clone();
        }
        let server = ServerData::new(
            default_name(),
            server_address.to_string(),
            ServerType::Other,
        );
        self.add(server.clone(), true);
        self.save();
        server
    }
    pub fn unhide(&mut self, ip: &str) -> Option<ServerData> {
        if let Some(pos) = self.hidden_servers.iter().position(|s| s.ip == ip) {
            let server = self.hidden_servers.remove(pos);
            self.servers.push(server.clone());
            Some(server)
        } else {
            None
        }
    }
    pub fn remove(&mut self, ip: &str) {
        if let Some(pos) = self.servers.iter().position(|s| s.ip == ip) {
            self.servers.remove(pos);
        } else if let Some(pos) = self.hidden_servers.iter().position(|s| s.ip == ip) {
            self.hidden_servers.remove(pos);
        }
    }
    pub fn add(&mut self, server: ServerData, hidden: bool) {
        if hidden {
            self.hidden_servers.insert(0, server);
            while self.hidden_servers.len() > 16 {
                self.hidden_servers.pop();
            }
        } else {
            self.servers.push(server);
        }
    }
    pub fn size(&self) -> usize {
        self.servers.len()
    }
    pub fn swap(&mut self, a: usize, b: usize) {
        if a < self.servers.len() && b < self.servers.len() {
            self.servers.swap(a, b);
            self.save();
        }
    }
    pub fn replace(&mut self, index: usize, server: ServerData) {
        if index < self.servers.len() {
            self.servers[index] = server;
        }
    }
}
