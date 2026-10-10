use std::fs;
use std::time::{Duration, SystemTime};

use zed_extension_api::{
    self as zed, settings::LspSettings, LanguageServerInstallationStatus as InstallationStatus,
    Result,
};

struct EncoreBinary {
    path: String,
    args: Vec<String>,
    env: zed::EnvVars,
}

struct EncoreExtension {
    cached_binary: Option<CachedBinary>,
}

struct CachedBinary {
    path: String,
    checked_at: SystemTime,
}

const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(15 * 60);

impl CachedBinary {
    fn is_fresh(&self, now: SystemTime) -> bool {
        now.duration_since(self.checked_at)
            .is_ok_and(|age| age < UPDATE_CHECK_INTERVAL)
    }

    fn exists(&self) -> bool {
        fs::metadata(&self.path).is_ok_and(|metadata| metadata.is_file())
    }
}

const ENCORE_LSP_BINARY: &str = "encore-lsp";
const ENCORE_RELEASE_REPOSITORY: &str = "encore-ecosystem/encore";

impl EncoreExtension {
    fn language_server_binary(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<EncoreBinary> {
        let settings =
            LspSettings::for_worktree(language_server_id.as_ref(), worktree).unwrap_or_default();
        let binary_settings = settings.binary;
        let mut command_env = worktree.shell_env();
        if let Some(overrides) = binary_settings
            .as_ref()
            .and_then(|binary| binary.env.clone())
        {
            for (name, value) in overrides {
                command_env.retain(|(existing, _)| existing != &name);
                command_env.push((name, value));
            }
        }
        let args = binary_settings
            .as_ref()
            .and_then(|binary| binary.arguments.clone())
            .unwrap_or_default();
        let configured_path = binary_settings.and_then(|binary| binary.path);
        // Explicit configuration wins. A broken environment override is an
        // error, not permission to silently launch an unrelated cached server.
        let path = match configured_path {
            Some(path) => Some(path),
            None => {
                environment_binary_path(&command_env)?.or_else(|| worktree.which(ENCORE_LSP_BINARY))
            }
        };
        if let Some(path) = path {
            return Ok(EncoreBinary {
                path,
                args,
                env: command_env,
            });
        }

        match self.zed_managed_binary_path(language_server_id) {
            Ok(path) => Ok(EncoreBinary {
                path,
                args,
                env: command_env,
            }),
            Err(error) => {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &InstallationStatus::Failed(error.clone()),
                );
                Err(error)
            }
        }
    }

    fn zed_managed_binary_path(
        &mut self,
        language_server_id: &zed::LanguageServerId,
    ) -> Result<String> {
        let now = SystemTime::now();
        if let Some(binary) = &self.cached_binary {
            if binary.is_fresh(now) && binary.exists() {
                return Ok(binary.path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &InstallationStatus::CheckingForUpdate,
        );
        let release = match zed::latest_github_release(
            ENCORE_RELEASE_REPOSITORY,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        ) {
            Ok(release) => release,
            Err(error) => {
                // Keep working offline, but retry on a bounded interval rather
                // than either checking per worktree or pinning forever.
                if let Some(binary) = &mut self.cached_binary {
                    if binary.exists() {
                        binary.checked_at = now;
                        zed::set_language_server_installation_status(
                            language_server_id,
                            &InstallationStatus::None,
                        );
                        return Ok(binary.path.clone());
                    }
                }
                return Err(error);
            }
        };
        let (os, architecture) = zed::current_platform();
        let arch = match architecture {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => {
                return Err("Encore does not publish a 32-bit Zed language server".into())
            }
        };
        let triple = match (os, architecture) {
            (zed::Os::Linux, _) => format!("{arch}-unknown-linux-gnu"),
            (zed::Os::Mac, _) => format!("{arch}-apple-darwin"),
            (zed::Os::Windows, zed::Architecture::X8664) => "x86_64-pc-windows-msvc".to_string(),
            (zed::Os::Windows, zed::Architecture::Aarch64) => "aarch64-w64-windows-gnu".to_string(),
            (zed::Os::Windows, zed::Architecture::X86) => unreachable!(),
        };
        let (package_name, asset_name) =
            release_layout(&release.version, &triple, os == zed::Os::Windows);
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("Encore release {} has no {asset_name}", release.version))?;
        let version_dir = format!("encore-lsp-{}-{triple}", release.version);
        let executable = if os == zed::Os::Windows {
            "encore-lsp.exe"
        } else {
            ENCORE_LSP_BINARY
        };
        let binary_path = format!("{version_dir}/{package_name}/bin/{executable}");

        if !fs::metadata(&binary_path).is_ok_and(|metadata| metadata.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &InstallationStatus::Downloading,
            );
            let file_type = if os == zed::Os::Windows {
                zed::DownloadedFileType::Zip
            } else {
                zed::DownloadedFileType::GzipTar
            };
            zed::download_file(&asset.download_url, &version_dir, file_type)
                .map_err(|error| format!("failed to download Encore LSP: {error}"))?;
            if os != zed::Os::Windows {
                zed::make_file_executable(&binary_path)
                    .map_err(|error| format!("failed to make Encore LSP executable: {error}"))?;
            }
        }
        if !fs::metadata(&binary_path).is_ok_and(|metadata| metadata.is_file()) {
            return Err(format!(
                "Encore LSP is absent after extracting {asset_name}"
            ));
        }

        zed::set_language_server_installation_status(language_server_id, &InstallationStatus::None);
        self.cached_binary = Some(CachedBinary {
            path: binary_path.clone(),
            checked_at: now,
        });
        Ok(binary_path)
    }
}

impl zed::Extension for EncoreExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            cached_binary: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        if language_server_id.as_ref() != "encore-lsp" {
            return Err(format!(
                "unknown Encore language server: {language_server_id}"
            ));
        }
        let binary = self.language_server_binary(language_server_id, worktree)?;
        Ok(zed::Command {
            command: binary.path,
            args: binary.args,
            env: binary.env,
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(
            LspSettings::for_worktree(language_server_id.as_ref(), worktree)?
                .initialization_options,
        )
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(LspSettings::for_worktree(language_server_id.as_ref(), worktree)?.settings)
    }
}

fn environment_value(environment: &zed::EnvVars, name: &str) -> Option<String> {
    environment
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value.clone())
}

fn environment_binary_path(environment: &zed::EnvVars) -> Result<Option<String>> {
    match environment_value(environment, "ENCORE_LSP_PATH").filter(|path| !path.is_empty()) {
        Some(path) if fs::metadata(&path).is_ok_and(|metadata| metadata.is_file()) => {
            Ok(Some(path))
        }
        Some(path) => Err(format!("ENCORE_LSP_PATH does not point to a file: {path}")),
        None => Ok(None),
    }
}

// Release tags select the version. Asset names remain stable for installers,
// while the archive's inner directory includes the complete release identity.
fn release_layout(release: &str, triple: &str, windows: bool) -> (String, String) {
    let version = release.trim_start_matches('v');
    let suffix = if windows { "zip" } else { "tar.gz" };
    (
        format!("encore-{version}-{triple}"),
        format!("encore-{triple}.{suffix}"),
    )
}

#[cfg(test)]
mod tests {
    use super::{environment_binary_path, release_layout, CachedBinary, UPDATE_CHECK_INTERVAL};
    use std::time::{Duration, SystemTime};

    #[test]
    fn named_release_uses_stable_asset_names() {
        for (triple, windows, suffix) in [
            ("x86_64-unknown-linux-gnu", false, "tar.gz"),
            ("aarch64-unknown-linux-gnu", false, "tar.gz"),
            ("x86_64-apple-darwin", false, "tar.gz"),
            ("aarch64-apple-darwin", false, "tar.gz"),
            ("x86_64-pc-windows-msvc", true, "zip"),
            ("aarch64-w64-windows-gnu", true, "zip"),
        ] {
            let (directory, asset) = release_layout("v0.1.2-neumann", triple, windows);
            assert_eq!(directory, format!("encore-0.1.2-neumann-{triple}"));
            assert_eq!(asset, format!("encore-{triple}.{suffix}"));
            assert_eq!(
                release_layout("0.1.2-neumann", triple, windows),
                (directory, asset)
            );
        }
    }

    #[test]
    fn managed_update_checks_are_bounded_but_not_permanent() {
        let checked_at = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let binary = CachedBinary {
            path: String::new(),
            checked_at,
        };
        assert!(binary.is_fresh(checked_at));
        assert!(binary.is_fresh(checked_at + UPDATE_CHECK_INTERVAL - Duration::from_secs(1)));
        assert!(!binary.is_fresh(checked_at + UPDATE_CHECK_INTERVAL));
        assert!(!binary.is_fresh(checked_at - Duration::from_secs(1)));
    }

    #[test]
    fn invalid_environment_override_does_not_silently_fall_back() {
        assert_eq!(environment_binary_path(&vec![]).unwrap(), None);
        assert_eq!(
            environment_binary_path(&vec![("ENCORE_LSP_PATH".into(), "".into())]).unwrap(),
            None
        );
        assert!(
            environment_binary_path(&vec![("ENCORE_LSP_PATH".into(), "\0invalid".into())]).is_err()
        );
        let path = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            environment_binary_path(&vec![("ENCORE_LSP_PATH".into(), path.clone())]).unwrap(),
            Some(path)
        );
    }
}

zed::register_extension!(EncoreExtension);
