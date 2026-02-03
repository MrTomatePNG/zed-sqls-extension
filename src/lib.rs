use serde_json::Value;
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
