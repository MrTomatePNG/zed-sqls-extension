use serde_json::Value;
use serde_yml;
use std::path::Path;
use zed_extension_api::{self as zed, GithubReleaseOptions, LanguageServerId, Result, Worktree};

struct SqlsExtension;
impl SqlsExtension {
    fn get_sqls_path_or_install(
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

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );

        let release = zed::latest_github_release(
            "sqls-server/sqls",
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (current_os, current_arch) = zed::current_platform();

        let os_str = match current_os {
            zed::Os::Mac => "darwin",
            zed::Os::Linux => "linux",
            zed::Os::Windows => "windows",
        };
        let arch_str = match current_arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => "x86",
        };

        let expected_asset_prefix = format!("sqls-{}-{}", os_str, arch_str);

        let asset = release
            .assets
            .iter()
            .find(|asset| {
                asset.name.contains(&expected_asset_prefix) && asset.name.ends_with(".zip")
            })
            .ok_or_else(|| {
                format!(
                    "Não foi encontrado um asset para a plataforma {}-{}. Prefixo esperado: {}",
                    os_str, arch_str, expected_asset_prefix
                )
            })?;

        let binary_filename_in_extension_dir = if current_os == zed::Os::Windows {
            "sqls.exe"
        } else {
            "sqls"
        };

        zed::download_file(
            &asset.download_url,
            binary_filename_in_extension_dir,
            zed::DownloadedFileType::Zip,
        )?;

        zed::make_file_executable(binary_filename_in_extension_dir)?;

        Ok(binary_filename_in_extension_dir.to_owned())
    }

    fn load_initialization_options(&self, worktree: &Worktree) -> Result<Option<Value>> {
        let root = worktree.root_path();
        let root_path = Path::new(&root);

        let config_path = root_path.join("config.yml");

        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = serde_yml::from_str::<Value>(&content) {
                return Ok(Some(serde_json::json!({ "sqls": config })));
            } else {
                eprintln!(
                    "Erro ao parsear {}: Conteúdo YAML inválido.",
                    config_path.display()
                );
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
        let sqls = self.get_sqls_path_or_install(language_server_id, worktree)?;

        Ok(zed::Command {
            command: sqls,
            args: Default::default(),
            env: Default::default(),
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
