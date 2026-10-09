//! Preserve collected command output before applying a user-selected context limit.
use crate::config::Config;
use crate::tools::context::ExecCommandToolOutput;
use codex_utils_output_truncation::approx_token_count;
use std::path::PathBuf;

async fn archive(home: &std::path::Path, bytes: &[u8], limit: usize) -> Option<PathBuf> {
    if approx_token_count(&String::from_utf8_lossy(bytes)) <= limit {
        return None;
    }
    let dir = home.join("tool-output").join("custom");
    let path = dir.join(format!("{}.log", uuid::Uuid::new_v4()));
    if let Err(err) = tokio::fs::create_dir_all(&dir).await {
        tracing::warn!(%err,"cannot create command output archive");
        return None;
    }
    if let Err(err) = tokio::fs::write(&path, bytes).await {
        tracing::warn!(%err,"cannot save command output archive");
        return None;
    }
    Some(path)
}

pub(super) async fn preserve_unified(
    config: &Config,
    mut response: ExecCommandToolOutput,
) -> ExecCommandToolOutput {
    if let Some(limit) = config.tool_output_token_limit {
        response.max_output_tokens = Some(
            response
                .max_output_tokens
                .map_or(limit, |requested| requested.min(limit)),
        );
        let effective_limit = response.max_output_tokens.unwrap_or(limit);
        if let Some(path) = archive(
            config.codex_home.as_path(),
            &response.raw_output,
            effective_limit,
        )
        .await
        {
            let label = if response.output_omitted_bytes.is_some() {
                "Collected portion (collection cap omitted earlier bytes)"
            } else {
                "Collected output"
            };
            let mut bytes = format!(
                "{label} saved to: {}\nRead the file selectively if more detail is needed.\n",
                path.display()
            )
            .into_bytes();
            bytes.extend_from_slice(&response.raw_output);
            response.raw_output = bytes;
        }
    }
    response
}

pub(super) async fn saved_output_notice(config: &Config, output: &str) -> Option<String> {
    let limit = config.tool_output_token_limit?;
    let path = archive(config.codex_home.as_path(), output.as_bytes(), limit).await?;
    Some(format!(
        "Collected output saved to: {}. Read selectively if needed.",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn custom_settings_output_archive_preserves_unicode_and_skips_short_output() {
        let home = tempfile::tempdir().unwrap();
        let full = "строка вывода\n".repeat(1000);
        let path = archive(home.path(), full.as_bytes(), 20).await.unwrap();
        assert_eq!(tokio::fs::read(path).await.unwrap(), full.as_bytes());
        assert!(archive(home.path(), b"short", 100).await.is_none());
    }
}
