use serde_json::Value;
use std::fs;
use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

struct SqlsBinary {
    path: String,
    args: Option<Vec<String>>,
}

struct SqlsExtension {
    cached_binary_path: Option<String>,
}

impl SqlsExtension {
    fn language_server_binary(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<SqlsBinary> {
        let settings =
            zed::settings::LspSettings::for_worktree(language_server_id.as_ref(), worktree);
        let binary = settings.ok().and_then(|settings| settings.binary);
        let args = binary.as_ref().and_then(|binary| binary.arguments.clone());
        let path = binary
            .and_then(|binary| binary.path)
            .or_else(|| worktree.which("sqls"))
            .unwrap_or_else(|| {
                self.zed_managed_binary_path(language_server_id)
                    .unwrap_or_default()
            });

        Ok(SqlsBinary { path, args })
    }

    fn zed_managed_binary_path(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
                return Ok(path.clone());
            }
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

        let (platform, arch) = zed::current_platform();
        let asset_name = format!(
            "sqls_{os}_{arch}",
            os = match platform {
                zed::Os::Mac => "darwin",
                zed::Os::Linux => "linux",
                zed::Os::Windows => "windows",
            },
            arch = match arch {
                zed::Architecture::Aarch64 => "arm64",
                zed::Architecture::X8664 => "amd64",
                zed::Architecture::X86 => return Err("unsupported platform x86".into()),
            },
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name.contains(&asset_name))
            .ok_or_else(|| format!("no asset found matching {asset_name:?}"))?;

        let version_dir = format!("sqls-{}", release.version);
        let binary_path = format!(
            "{version_dir}/sqls{extension}",
            extension = match platform {
                zed::Os::Windows => ".exe",
                _ => "",
            },
        );

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &version_dir,
                zed::DownloadedFileType::Zip,
            )
            .map_err(|e| format!("failed to download file: {e}"))?;

            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list directory: {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load entry: {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
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
        let sqls_binary = self.language_server_binary(language_server_id, worktree)?;

        let mut args = sqls_binary.args.unwrap_or_default();
        let root_path = worktree.root_path();
        args.push("-l".to_string());
        args.push(format!("{}/sqls.log", root_path));

        let possible_configs = [
            format!("{}/.sqls/config.yml", root_path),
            format!("{}/config.yml", root_path),
        ];

        for config_path in possible_configs {
            if fs::metadata(&config_path).is_ok_and(|stat| stat.is_file()) {
                args.push("-c".to_string());
                args.push(config_path);
                break;
            }
        }

        Ok(zed::Command {
            command: sqls_binary.path,
            args,
            env: vec![],
        })
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        let settings =
            zed::settings::LspSettings::for_worktree(language_server_id.as_ref(), worktree);
        Ok(settings.ok().and_then(|settings| settings.settings))
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        let settings =
            zed::settings::LspSettings::for_worktree(language_server_id.as_ref(), worktree);

        if let Ok(settings) = &settings {
            if let Some(options) = &settings.initialization_options {
                return Ok(Some(options.clone()));
            }
        }

        // Se não houver config manual, injetamos uma configuração padrão
        // apontando para um data.db absoluto na raiz do projeto.
        // Isso ajuda o sqls a não se perder no caminho relativo.
        let db_path = format!("{}/data.db", worktree.root_path());

        Ok(Some(serde_json::json!({
            "connections": [
                {   "alias": "main",
                    "driver": "sqlite3",
                    "dataSourceName": db_path
                }
            ]
        })))
    }
}

zed::register_extension!(SqlsExtension);
