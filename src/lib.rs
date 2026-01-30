use serde_json::Value;
use std::{fs, path::Path};
use zed_extension_api::{
    self as zed,
    lsp::{Completion, CompletionKind, Symbol},
    CodeLabel, CodeLabelSpan, LanguageServerId, Result, Worktree,
};

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
        let root_path = worktree.root_path();
        let mut args = sqls_binary.args.unwrap_or_default();

        // -t: Trace para ver o JSON-RPC no log do Zed
        args.push("-t".to_string());

        let possible_relative_paths = ["/.sqls/config.yml", "/config.yml"];

        let mut found_config = Some(".sqls/config.yml".to_string());

        for relative_path in possible_relative_paths {
            let full_path = Path::new(&root_path).join(relative_path);
            if full_path.exists() {
                if let Some(path_str) = full_path.to_str() {
                    found_config = Some(path_str.to_string());
                    break;
                }
            }
        }

        if let Some(path) = found_config {
            args.push("-c".to_string());
            args.push(path);
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
        Ok(settings.ok().and_then(|s| s.settings))
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        // Pega o que o usuário definiu no settings.json do Zed
        let user_settings =
            zed::settings::LspSettings::for_worktree(language_server_id.as_ref(), worktree)
                .ok()
                .and_then(|s| s.initialization_options)
                .unwrap_or(serde_json::json!({}));

        // Forçamos o suporte a CodeActionLiteralSupport no handshake inicial
        let mut options = user_settings.as_object().cloned().unwrap_or_default();

        // O sqls às vezes precisa saber que o cliente suporta comandos específicos
        options.insert(
            "codeAction".into(),
            serde_json::json!({
                "isPreferred": true,
                "kind": "refactor"
            }),
        );

        Ok(Some(serde_json::Value::Object(options)))
    }

    fn label_for_completion(
        &self,
        _language_server_id: &LanguageServerId,
        completion: Completion,
    ) -> Option<CodeLabel> {
        let kind = completion.kind?;

        match kind {
            // TABELAS: O sqls geralmente usa Class ou Struct
            CompletionKind::Class | CompletionKind::Struct => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![
                        CodeLabelSpan::literal("TABLE ".to_string(), Some("keyword".into())),
                        CodeLabelSpan::literal(label.clone(), None),
                    ],
                    filter_range: (0..label.len()).into(),
                    code: format!("TABLE {label}"),
                })
            }

            // COLUNAS: O sqls usa Field ou Property
            CompletionKind::Field | CompletionKind::Property => {
                let label = completion.label;
                let detail = completion.detail.unwrap_or_default();
                let code = format!("{label} {detail}");

                Some(CodeLabel {
                    spans: vec![
                        CodeLabelSpan::literal(label.clone(), Some("property".into())),
                        CodeLabelSpan::literal(" ".to_string(), None),
                        CodeLabelSpan::literal(detail, Some("type".into())),
                    ],
                    filter_range: (0..label.len()).into(),
                    code,
                })
            }

            // PALAVRAS-CHAVE: SELECT, FROM, JOIN
            CompletionKind::Keyword => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(
                        label.clone(),
                        Some("keyword".into()),
                    )],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // O resto você pode deixar o Zed tratar ou retornar None para o padrão
            _ => None,
        }
    }

    fn label_for_symbol(
        &self,
        _language_server_id: &LanguageServerId,
        symbol: Symbol,
    ) -> Option<CodeLabel> {
        Some(CodeLabel {
            code: symbol.name.clone(),
            spans: vec![CodeLabelSpan::literal(symbol.name.clone(), None)],
            filter_range: (0..symbol.name.len()).into(),
        })
    }
}

zed::register_extension!(SqlsExtension);
