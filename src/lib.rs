use serde_json::Value;
use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

struct SqlsExtension {
    cached_binary_path: Option<String>,
}
impl SqlsExtension {
    fn get_sqls_path_or_install(
        &mut self,
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

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            "sqls-server/sqls",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, _arch) = zed::current_platform();

        let os_str = match platform {
            zed::Os::Mac => "darwin",
            zed::Os::Linux => "linux",
            zed::Os::Windows => "windows",
        };

        let asset_name = format!("sqls-{}", os_str);

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name.starts_with(&asset_name))
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = format!("sqls-{}", release.version);
        let binary_path = match platform {
            zed::Os::Windows => format!("{}/sqls.exe", version_dir),
            _ => format!("{}/sqls", version_dir),
        };

        if !std::fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &version_dir,
                zed::DownloadedFileType::Zip,
            )?;

            zed::make_file_executable(&binary_path)?;

            self.remove_outdated_versions(&version_dir)?;
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }

    fn remove_outdated_versions(&self, current_version_dir: &str) -> Result<()> {
        let versions_dir = ".";
        if let Ok(entries) = std::fs::read_dir(versions_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        let path = entry.path();
                        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                            if dir_name.starts_with("sqls-") && dir_name != current_version_dir {
                                let _ = std::fs::remove_dir_all(&path);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl zed::Extension for SqlsExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let sqls = self.get_sqls_path_or_install(language_server_id, worktree)?;

        Ok(zed::Command {
            command: sqls,
            args: vec!["-t".into()],
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        let settings =
            zed::settings::LspSettings::for_worktree(language_server_id.as_ref(), worktree)?;
        Ok(settings.initialization_options)
    }
    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        let settings =
            zed::settings::LspSettings::for_worktree(language_server_id.as_ref(), worktree)?;
        Ok(settings.settings)
    }
}

zed::register_extension!(SqlsExtension);
