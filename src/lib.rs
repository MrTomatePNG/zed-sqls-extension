use serde_json::Value;
use std::fs;
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

        Ok(zed::Command {
            command: sqls_binary.path,
            args: sqls_binary.args.unwrap_or(vec!["-t".into()]),
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

    fn label_for_completion(
        &self,
        _language_server_id: &LanguageServerId,
        completion: Completion,
    ) -> Option<CodeLabel> {
        let kind = completion.kind?;

        match kind {
            // TABELAS
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

            // COLUNAS
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

            // PALAVRAS-CHAVE
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

            // MÉTODOS
            CompletionKind::Method => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("method".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label.clone(),
                })
            }

            // FUNÇÕES
            CompletionKind::Function => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("function".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label.clone(),
                })
            }

            // CONSTRUTORES
            CompletionKind::Constructor => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("constructor".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label.clone(),
                })
            }

            // VARIÁVEIS
            CompletionKind::Variable => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("variable".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // INTERFACES
            CompletionKind::Interface => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("interface".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // MÓDULOS
            CompletionKind::Module => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("module".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // UNIDADES
            CompletionKind::Unit => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("unit".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // SNIPPETS
            CompletionKind::Snippet => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("snippet".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // CORES
            CompletionKind::Color => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("color".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // ARQUIVOS
            CompletionKind::File => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("file".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // REFERÊNCIAS
            CompletionKind::Reference => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("reference".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // PASTAS
            CompletionKind::Folder => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("folder".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // MEMBROS DE ENUM
            CompletionKind::EnumMember => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("enum_member".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // EVENTOS
            CompletionKind::Event => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("event".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // OPERADORES
            CompletionKind::Operator => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(&label, Some("operator".into()))],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // PARÂMETROS DE TIPO
            CompletionKind::TypeParameter => {
                let label = completion.label;
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::literal(
                        &label,
                        Some("type_parameter".into()),
                    )],
                    filter_range: (0..label.len()).into(),
                    code: label,
                })
            }

            // Outros tipos não reconhecidos
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
