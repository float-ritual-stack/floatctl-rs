use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use crate::filters::FilterContext;
use crate::model::{Conversation, Message};
use crate::slug::strip_leading_date;

pub fn write_conversation_md(
    conversation: &Conversation,
    filter_ctx: &FilterContext,
    artifact_paths: Option<&[Vec<String>]>,
    path: &Path,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    let mut file =
        fs::File::create(path).with_context(|| format!("failed to create {}", path.display()))?;

    writeln!(file, "---")?;
    writeln!(file, "id: {}", conversation.conv_id)?;
    writeln!(file, "source: {}", conversation.source.as_str())?;
    if let Some(model) = conversation.model.as_ref() {
        writeln!(file, "model: {model}")?;
    }
    writeln!(
        file,
        "created: {}",
        filter_ctx
            .display_timestamp(Some(conversation.created))
            .unwrap_or_else(|| conversation.created.to_rfc3339())
    )?;
    if let Some(updated) = conversation.updated {
        writeln!(
            file,
            "updated: {}",
            filter_ctx
                .display_timestamp(Some(updated))
                .unwrap_or_else(|| updated.to_rfc3339())
        )?;
    }
    if !conversation.participants.is_empty() {
        let participants = conversation
            .participants
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(file, "participants: [{participants}]")?;
    }
    writeln!(file, "---\n")?;

    let heading = conversation
        .title
        .as_deref()
        .map(|title| {
            let stripped = strip_leading_date(title);
            let trimmed = stripped.trim();
            if trimmed.is_empty() {
                format!("Conversation {}", conversation.conv_id)
            } else {
                trimmed.to_string()
            }
        })
        .unwrap_or_else(|| format!("Conversation {}", conversation.conv_id));

    writeln!(file, "# {heading}\n")?;

    if let Some(summary) = conversation.summary.as_ref() {
        if !summary.trim().is_empty() {
            writeln!(file, ">{summary}\n")?;
        }
    }

    for (idx, message) in conversation.messages.iter().enumerate() {
        let artifacts_for_message = artifact_paths.and_then(|paths| paths.get(idx));
        write_message(
            idx + 1,
            message,
            artifacts_for_message,
            filter_ctx,
            &mut file,
        )?;
    }

    Ok(())
}

fn write_message(
    index: usize,
    message: &Message,
    artifact_paths: Option<&Vec<String>>,
    filter_ctx: &FilterContext,
    file: &mut fs::File,
) -> Result<()> {
    let role = message.role.as_str();
    let timestamp = filter_ctx.display_timestamp(message.timestamp);
    let heading = match timestamp {
        Some(ts) => format!("## Message {index} — {role} — {ts}"),
        None => format!("## Message {index} — {role}"),
    };
    writeln!(file, "{heading}\n")?;

    for channel in &message.channels {
        match channel.channel.as_str() {
            "message" | "reply" | "reasoning" | "system" | "tool" | "other" => {
                writeln!(file, "**{}**\n", channel.channel.to_uppercase())?;
                write_content_block(&channel.text, file)?;
            }
            _ => {
                writeln!(file, "**{}**\n", channel.channel)?;
                write_content_block(&channel.text, file)?;
            }
        }
    }

    if !message.attachments.is_empty() {
        writeln!(file, "\n**Attachments**")?;
        for attachment in &message.attachments {
            let name = attachment.name.as_deref().unwrap_or("(unnamed attachment)");
            let mut parts = vec![format!("- {name}")];
            if let Some(uri) = attachment.uri.as_ref() {
                parts.push(format!("uri: {uri}"));
            }
            if let Some(mime) = attachment.mime.as_ref() {
                parts.push(format!("mime: {mime}"));
            }
            writeln!(file, "{}", parts.join(", "))?;
        }
        writeln!(file)?;
    }

    if !message.tool_calls.is_empty() {
        writeln!(file, "**Tool Calls**")?;
        for call in &message.tool_calls {
            writeln!(file, "- `{}` args: `{}`", call.name, call.args)?;
        }
        writeln!(file)?;
    }

    if !message.artifacts.is_empty() {
        writeln!(file, "**Artifacts**")?;
        for artifact in &message.artifacts {
            let kind = artifact.kind.as_deref().unwrap_or("artifact");
            writeln!(file, "- kind: {kind}")?;
        }
        writeln!(file)?;
    }

    if let Some(paths) = artifact_paths {
        if !paths.is_empty() {
            writeln!(file, "**Artifact Files**")?;
            for path in paths {
                writeln!(file, "- extracted to {path}")?;
            }
            writeln!(file)?;
        }
    }

    Ok(())
}

fn write_content_block(content: &str, file: &mut fs::File) -> Result<()> {
    if content.contains("```") {
        writeln!(file, "~~~")?;
        writeln!(file, "{content}")?;
        writeln!(file, "~~~\n")?;
    } else {
        writeln!(file, "```\n{content}\n```\n")?;
    }
    Ok(())
}
