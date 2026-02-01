use serde_json::Value;
use std::path::Path;
use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

const PROXY: &str = include_str!("proxy.mjs");

struct SqlsExtension;
impl SqlsExtension {
    async fn get_sqls_path_or_install(
        &self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<String> {
        let settings =
            zed::settings::LspSettings::for_worktree(language_server_id.as_ref(), worktree);

        if let Some(path) = settings
            .ok()
            .and_then(|s| s.binary)
            .and_then(|b| b.path)
            .or_else(|| worktree.which("sqls"))
        {
            return Ok(path);
        }

        // If not found, proceed with installation
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        let release = zed::latest_github_release("sqls-server/sqls".to_string()).await?;
        let (current_os, current_arch) = zed::current_platform();
        let asset_name = format!("sqls-{}-{}-{}", release.version, current_os, current_arch);

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name.contains(&asset_name))
            .ok_or_else(|| format!("no asset found for {:?}", zed::current_platform()))?;

        let downloaded_path = zed::download_file(
            &asset.download_url,
            format!("sqls-{}", release.version).to_string(),
        )
        .await?;
        zed::make_file_executable(&downloaded_path).await?;

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Installed,
        );

        Ok(downloaded_path.to_string())
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

    async fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let sqls_path = self
            .get_sqls_path_or_install(language_server_id, worktree)
            .await?;

        Ok(zed::Command {
            command: zed::node_binary_path()?,
            args: vec!["-e".into(), PROXY.into(), sqls_path],
            env: Default::default(),
        })
    }

    async fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        self.load_initialization_options(worktree)
    }
}

zed::register_extension!(SqlsExtension);
