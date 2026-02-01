use serde_json::Value;
use std::path::Path;
use zed_extension_api::{self as zed, node_binary_path, LanguageServerId, Result, Worktree};

struct SqlsExtension;

impl SqlsExtension {
    fn get_sqls_path(
        &self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<String> {
        let settings =
            zed::settings::LspSettings::for_worktree(language_server_id.as_ref(), worktree);

        let path = settings
            .ok()
            .and_then(|s| s.binary)
            .and_then(|b| b.path)
            .or_else(|| worktree.which("sqls"))
            .unwrap_or_else(|| "~/go/bin/sqls".to_string());

        Ok(path)
    }

    fn load_initialization_options(&self, worktree: &Worktree) -> Result<Option<Value>> {
        let root = worktree.root_path();
        let root_path = Path::new(&root);

        let config_paths = [root_path.join(".sqlsrc.json"), root_path.join("sqls.json")];

        for path in &config_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str::<Value>(&content) {
                    return Ok(Some(serde_json::json!({ "sqls": config })));
                }
            }
        }
        Ok(None)
    }
}

impl zed::Extension for SqlsExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let sqls_path = self.get_sqls_path(language_server_id, worktree)?;
        let proxy_code = include_str!("proxy.mjs");

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec!["-e".into(), proxy_code.into(), sqls_path],
            env: vec![],
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        self.load_initialization_options(worktree)
    }
}

zed::register_extension!(SqlsExtension);
