//! `loom search` — rank repository skills against a query and print them as an
//! aligned table of name, version, source, and description.

use crate::commands::open_repo;
use crate::config::Config;
use crate::manifest::Manifest;
use crate::output;
use anyhow::Result;
use colored::Colorize;

/// Hard caps so one long field can't starve the description column.
const NAME_CAP: usize = 28;
const SOURCE_CAP: usize = 18;
/// Description never narrower than this, even in a cramped terminal.
const DESC_MIN: usize = 24;
/// Assumed width when stdout is not a terminal (piped/redirected).
const FALLBACK_COLS: usize = 100;

pub fn run(query: &str) -> Result<()> {
    let config = Config::load()?;
    let repo = open_repo(&config);
    let hits = repo.search(query)?;

    if hits.is_empty() {
        output::warn(&format!("no skills matching '{query}'"));
        return Ok(());
    }

    let rows: Vec<Row> = hits
        .iter()
        .map(|(entry, _)| Row::from_manifest(&entry.manifest))
        .collect();

    // Size each fixed column to its widest cell (bounded by the caps), then
    // give whatever horizontal space remains to the description.
    let name_w = col_width("NAME", rows.iter().map(|r| r.name.as_str()), NAME_CAP);
    let ver_w = col_width("VER", rows.iter().map(|r| r.version.as_str()), NAME_CAP);
    let src_w = col_width("SOURCE", rows.iter().map(|r| r.source.as_str()), SOURCE_CAP);

    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
    let cols = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(FALLBACK_COLS);
    // Fixed columns plus two spaces of gap between each of the four columns.
    let used = name_w + ver_w + src_w + 3 * GAP.len();
    let desc_w = cols.saturating_sub(used).max(DESC_MIN);

    let header = format!(
        "{}{GAP}{}{GAP}{}{GAP}{}",
        pad("NAME", name_w),
        pad("VER", ver_w),
        pad("SOURCE", src_w),
        "DESCRIPTION",
    );
    println!("{}", header.dimmed());

    for r in &rows {
        // Only truncate the description when writing to a real terminal; a pipe
        // gets the full text so nothing is silently lost downstream.
        let desc = if is_tty { fit(&r.description, desc_w) } else { r.description.clone() };
        println!(
            "{}{GAP}{}{GAP}{}{GAP}{}",
            pad(&r.name, name_w).green().bold(),
            pad(&r.version, ver_w).dimmed(),
            pad(&r.source, src_w).cyan(),
            desc,
        );
    }

    output::detail("tip", "`loom info <name>` for full details");
    Ok(())
}

/// Column separator.
const GAP: &str = "  ";

/// One rendered search result.
struct Row {
    name: String,
    version: String,
    source: String,
    description: String,
}

impl Row {
    fn from_manifest(m: &Manifest) -> Row {
        Row {
            name: m.name.clone(),
            version: m.version.clone(),
            source: source_label(m),
            description: m.description.replace('\n', " ").trim().to_string(),
        }
    }
}

/// The owning org/user of a skill's source, e.g. `anthropics` from a GitHub URL.
/// Falls back to the raw host-less URL when no owner can be parsed.
fn source_label(m: &Manifest) -> String {
    let url = m
        .source
        .url
        .trim_end_matches('/')
        .trim_end_matches(".git");
    let hostless = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("git@")
        .replace(':', "/");
    let mut parts = hostless.split('/').filter(|s| !s.is_empty());
    let _host = parts.next();
    parts.next().map(String::from).unwrap_or_else(|| hostless.clone())
}

/// Width for a fixed column: the widest of its header and cells, capped.
fn col_width<'a>(header: &str, cells: impl Iterator<Item = &'a str>, cap: usize) -> usize {
    let widest = cells.map(display_len).max().unwrap_or(0);
    widest.max(header.len()).min(cap)
}

/// Left-pad `s` to `w`, truncating with an ellipsis if it overruns.
fn pad(s: &str, w: usize) -> String {
    let s = fit(s, w);
    let len = display_len(&s);
    if len < w {
        format!("{s}{}", " ".repeat(w - len))
    } else {
        s
    }
}

/// Truncate `s` to at most `w` display columns, appending `…` when clipped.
fn fit(s: &str, w: usize) -> String {
    if display_len(s) <= w {
        return s.to_string();
    }
    if w == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in s.chars() {
        if used + 1 >= w {
            break;
        }
        out.push(ch);
        used += 1;
    }
    out.push('…');
    out
}

/// Character count, used as a proxy for display width (skill metadata is
/// effectively ASCII, so this stays exact in practice).
fn display_len(s: &str) -> usize {
    s.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{Manifest, Source, SourceKind};

    fn manifest_with_source(url: &str) -> Manifest {
        Manifest {
            name: "x".into(),
            version: "0.1.0".into(),
            description: "d".into(),
            homepage: "https://example.com".into(),
            license: None,
            authors: vec![],
            keywords: vec![],
            compatibility: vec![],
            source: Source {
                kind: SourceKind::Git,
                url: url.into(),
                ref_: None,
                sha256: None,
                subdir: None,
            },
            install: Default::default(),
        }
    }

    #[test]
    fn fit_pads_short_and_truncates_long() {
        assert_eq!(pad("go", 5), "go   ");
        assert_eq!(fit("hello", 5), "hello");
        assert_eq!(fit("kubernetes", 5), "kube…");
        assert_eq!(display_len(&pad("kubernetes-helm", 8)), 8);
    }

    #[test]
    fn source_label_extracts_owner() {
        assert_eq!(source_label(&manifest_with_source("https://github.com/obra/superpowers")), "obra");
        assert_eq!(
            source_label(&manifest_with_source("https://github.com/anthropics/skills.git")),
            "anthropics"
        );
        assert_eq!(
            source_label(&manifest_with_source("git@github.com:pbakaus/impeccable.git")),
            "pbakaus"
        );
    }
}
