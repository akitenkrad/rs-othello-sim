//! `fetch` subcommand: download public game-record datasets.
//!
//! Currently only WTHOR (French Othello Federation) is supported. See
//! `docs/external-data.md` and `docs/cli-usage.md` for design details.

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args as ClapArgs, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{self, File};
use std::io::{self, BufWriter, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Default WTHOR URL pattern. The `{YEAR}` placeholder is substituted with a
/// 4-digit year.
///
/// If the layout of ffothello.org changes, this can be overridden via the
/// `OTHELLO_WTHOR_URL_PATTERN` environment variable or the `--url-pattern`
/// flag.
pub const DEFAULT_WTHOR_URL_PATTERN: &str = "https://www.ffothello.org/wthor/wth_{YEAR}.zip";

/// Environment variable used to override the WTHOR URL pattern.
pub const WTHOR_URL_ENV: &str = "OTHELLO_WTHOR_URL_PATTERN";

/// Top-level arguments for the `fetch` subcommand.
#[derive(Debug, ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: FetchCommand,
}

/// Subcommands of `fetch`.
#[derive(Debug, Subcommand)]
pub enum FetchCommand {
    /// List the supported datasets.
    List,
    /// Download a WTHOR archive from FFO.
    Wthor(WthorArgs),
}

/// Arguments for `fetch wthor`.
#[derive(Debug, ClapArgs)]
pub struct WthorArgs {
    /// Fetch a single year (e.g. `--year 2023`). Mutually exclusive with `--years`.
    #[arg(long, conflicts_with = "years")]
    pub year: Option<u32>,

    /// Fetch a range, inclusive on both ends (e.g. `--years 2020..2023`).
    /// Mutually exclusive with `--year`.
    #[arg(long, conflicts_with = "year")]
    pub years: Option<String>,

    /// Destination directory for the extracted files (defaults to `data/wthor/`).
    #[arg(long, default_value = "data/wthor/")]
    pub dest: PathBuf,

    /// Overwrite even if the destination files already exist.
    #[arg(long, default_value_t = false)]
    pub force: bool,

    /// Keep the downloaded zip archive instead of deleting it.
    #[arg(long, default_value_t = false)]
    pub keep_archive: bool,

    /// Custom URL template (must contain the `{YEAR}` placeholder, e.g.
    /// `https://example.com/wth_{YEAR}.zip`). Precedence: CLI > env
    /// (`OTHELLO_WTHOR_URL_PATTERN`) > built-in default.
    #[arg(long)]
    pub url_pattern: Option<String>,

    /// Suppress the progress bar (useful when piping). Otherwise shown when
    /// stderr is a TTY.
    #[arg(long, default_value_t = false)]
    pub no_progress: bool,
}

/// Shared configuration consumed by `fetch_wthor_year` and friends.
#[derive(Debug, Clone)]
pub struct FetchOptions {
    /// URL template containing the `{YEAR}` placeholder.
    pub url_pattern: String,
    /// Destination directory.
    pub dest: PathBuf,
    /// Whether to overwrite existing files.
    pub force: bool,
    /// Whether to keep the downloaded zip.
    pub keep_archive: bool,
    /// Whether to display a progress bar.
    pub show_progress: bool,
}

/// Entry point for `fetch`.
pub fn run(args: Args) -> Result<()> {
    match args.command {
        FetchCommand::List => print_list(&mut io::stdout().lock()),
        FetchCommand::Wthor(a) => run_wthor(a),
    }
}

/// Output helper used by `fetch list`.
pub fn print_list<W: Write>(out: &mut W) -> Result<()> {
    writeln!(out, "Supported datasets:")?;
    writeln!(out)?;
    writeln!(
        out,
        "  wthor    French Othello Federation tournament database (.wtb binary)"
    )?;
    writeln!(out, "           Per-year archives, ~1977-present")?;
    writeln!(
        out,
        "           Source: https://www.ffothello.org/informatique/la-base-wthor/"
    )?;
    writeln!(
        out,
        "           Usage: othello-cli fetch wthor --year 2023 --dest data/wthor/"
    )?;
    writeln!(out)?;
    writeln!(out, "For details, see docs/external-data.md.")?;
    Ok(())
}

/// Entry point for `fetch wthor`.
pub fn run_wthor(args: WthorArgs) -> Result<()> {
    let years = resolve_years(args.year, args.years.as_deref())?;
    let url_pattern = resolve_url_pattern(args.url_pattern.as_deref());
    validate_url_pattern(&url_pattern)?;

    let show_progress = !args.no_progress && std::io::stderr().is_terminal();
    let opts = FetchOptions {
        url_pattern,
        dest: args.dest.clone(),
        force: args.force,
        keep_archive: args.keep_archive,
        show_progress,
    };

    fs::create_dir_all(&opts.dest)
        .with_context(|| format!("failed to create destination: {}", opts.dest.display()))?;

    for year in years {
        fetch_wthor_year(year, &opts)?;
    }
    Ok(())
}

/// Download and extract a single year of the WTHOR archive.
pub fn fetch_wthor_year(year: u32, opts: &FetchOptions) -> Result<()> {
    let url = expand_url(&opts.url_pattern, year)?;
    let dest_dir = &opts.dest;
    fs::create_dir_all(dest_dir)
        .with_context(|| format!("failed to create destination: {}", dest_dir.display()))?;

    let archive_path = dest_dir.join(format!("wth_{year}.zip"));
    let extracted_marker = dest_dir.join(format!("WTH_{year}.wtb"));
    let extracted_marker_lower = dest_dir.join(format!("wth_{year}.wtb"));

    if (extracted_marker.exists() || extracted_marker_lower.exists()) && !opts.force {
        info!(
            year = year,
            "wthor archive already extracted, skipping (use --force)"
        );
        let shown = if extracted_marker_lower.exists() {
            extracted_marker_lower.display().to_string()
        } else {
            extracted_marker.display().to_string()
        };
        eprintln!("[{year}] {shown} already exists, skipping (use --force)");
        return Ok(());
    }

    info!(year = year, url = %url, "downloading wthor archive");
    eprintln!("[{year}] downloading {url}");
    download_with_progress(&url, &archive_path, opts.show_progress)
        .with_context(|| format!("failed to download {url}"))?;

    info!(year = year, archive = %archive_path.display(), "extracting wthor archive");
    let extracted = extract_zip(&archive_path, dest_dir)
        .with_context(|| format!("failed to extract {}", archive_path.display()))?;

    if !opts.keep_archive {
        fs::remove_file(&archive_path)
            .with_context(|| format!("failed to remove archive: {}", archive_path.display()))?;
    }

    info!(
        year = year,
        files = extracted,
        dest = %dest_dir.display(),
        "wthor fetch complete"
    );
    eprintln!(
        "[{year}] saved {} file(s) to {}",
        extracted,
        dest_dir.display()
    );
    Ok(())
}

/// Interpret `--year` / `--years` and return the list of years to process.
fn resolve_years(year: Option<u32>, years: Option<&str>) -> Result<Vec<u32>> {
    match (year, years) {
        (Some(_), Some(_)) => Err(anyhow!("--year and --years are mutually exclusive")),
        (Some(y), None) => Ok(vec![y]),
        (None, Some(s)) => {
            let (a, b) = parse_year_range(s)?;
            Ok((a..=b).collect())
        }
        (None, None) => Err(anyhow!(
            "either --year or --years is required (e.g. --year 2023 or --years 2020..2023)"
        )),
    }
}

/// Parse a string of the form `"2020..2023"` into `(2020, 2023)` (inclusive on both ends).
pub fn parse_year_range(s: &str) -> Result<(u32, u32)> {
    let parts: Vec<&str> = s.split("..").collect();
    if parts.len() != 2 {
        bail!("invalid --years value: {s:?} (expected `START..END`)");
    }
    let a: u32 = parts[0]
        .trim()
        .parse()
        .map_err(|e| anyhow!("invalid start year in {s:?}: {e}"))?;
    let b: u32 = parts[1]
        .trim()
        .parse()
        .map_err(|e| anyhow!("invalid end year in {s:?}: {e}"))?;
    if a > b {
        bail!("invalid --years value: start ({a}) is after end ({b})");
    }
    Ok((a, b))
}

/// Resolve the URL pattern using CLI > env > default precedence.
fn resolve_url_pattern(cli: Option<&str>) -> String {
    if let Some(s) = cli {
        return s.to_string();
    }
    if let Ok(s) = std::env::var(WTHOR_URL_ENV)
        && !s.is_empty()
    {
        return s;
    }
    DEFAULT_WTHOR_URL_PATTERN.to_string()
}

/// Verify that the URL pattern contains the `{YEAR}` placeholder.
fn validate_url_pattern(pattern: &str) -> Result<()> {
    if !pattern.contains("{YEAR}") {
        bail!(
            "URL pattern {pattern:?} does not contain the `{{YEAR}}` placeholder; \
             pass `--url-pattern 'https://.../wth_{{YEAR}}.zip'` or set {WTHOR_URL_ENV}"
        );
    }
    Ok(())
}

/// Substitute `{YEAR}` with the 4-digit year and return the URL.
pub fn expand_url(pattern: &str, year: u32) -> Result<String> {
    if !pattern.contains("{YEAR}") {
        bail!("URL pattern {pattern:?} does not contain the `{{YEAR}}` placeholder");
    }
    Ok(pattern.replace("{YEAR}", &format!("{year:04}")))
}

/// HTTP GET to file. If `Content-Length` is available, show a progress bar.
fn download_with_progress(url: &str, dest: &Path, show_progress: bool) -> Result<u64> {
    let resp = ureq::get(url)
        .call()
        .map_err(|e| classify_ureq_error(url, e))?;

    let total: Option<u64> = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok());

    let bar = if show_progress {
        let pb = match total {
            Some(n) => {
                let pb = ProgressBar::new(n);
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}",
                    )
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("##-"),
                );
                pb
            }
            None => {
                let pb = ProgressBar::new_spinner();
                pb.set_style(
                    ProgressStyle::with_template(
                        "[{elapsed_precise}] {spinner} {bytes} ({bytes_per_sec}) {msg}",
                    )
                    .unwrap_or_else(|_| ProgressStyle::default_spinner()),
                );
                pb
            }
        };
        pb.set_message(dest.file_name().map_or_else(
            || "downloading".to_string(),
            |n| n.to_string_lossy().to_string(),
        ));
        Some(pb)
    } else {
        None
    };

    let file = File::create(dest)
        .with_context(|| format!("failed to create archive file: {}", dest.display()))?;
    let mut writer = BufWriter::new(file);
    let mut reader = resp.into_reader();

    let mut total_read: u64 = 0;
    let mut buf = [0u8; 16 * 1024];
    loop {
        let n = reader.read(&mut buf).context("network read failed")?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n]).context("file write failed")?;
        total_read += n as u64;
        if let Some(pb) = bar.as_ref() {
            pb.set_position(total_read);
        }
    }
    writer.flush().context("file flush failed")?;

    if let Some(pb) = bar {
        pb.finish_with_message("done");
    }
    info!(url = url, bytes = total_read, "download complete");
    Ok(total_read)
}

/// Format a `ureq::Error` into a user-friendly anyhow error, distinguishing
/// structured failures (e.g. HTTP 404) from transport errors.
fn classify_ureq_error(url: &str, err: ureq::Error) -> anyhow::Error {
    match err {
        ureq::Error::Status(404, _) => anyhow!(
            "The URL `{url}` returned 404. The WTHOR archive layout may have changed. \
             Please check `https://www.ffothello.org/informatique/la-base-wthor/` and \
             pass `--url-pattern` (or set the environment variable `{WTHOR_URL_ENV}`) if needed."
        ),
        ureq::Error::Status(code, _) => anyhow!(
            "The URL `{url}` returned HTTP {code}. \
             If the WTHOR archive layout has changed, pass `--url-pattern` or set `{WTHOR_URL_ENV}`."
        ),
        ureq::Error::Transport(t) => anyhow!("network error fetching `{url}`: {t}"),
    }
}

/// Extract the zip directly into `dest_dir`, keeping only `.wtb` / `.JOU` /
/// `.TOU` entries. If no `.wtb` entry is present, a warning is emitted and
/// every entry is extracted. Returns the number of files extracted.
fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<usize> {
    let file = File::open(zip_path)
        .with_context(|| format!("failed to open archive: {}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("failed to read zip: {}", zip_path.display()))?;

    // 第 1 パス: `.wtb` の存在を判定．
    let mut has_wtb = false;
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .with_context(|| format!("zip entry {i} unreadable"))?;
        if let Some(name) = entry.enclosed_name() {
            if has_wtb_extension(&name) {
                has_wtb = true;
                break;
            }
        }
    }
    if !has_wtb {
        warn!(
            archive = %zip_path.display(),
            "no .wtb entry found in archive; extracting all files"
        );
    }

    // 第 2 パス: 抽出．
    let mut extracted: usize = 0;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .with_context(|| format!("zip entry {i} unreadable"))?;

        // ZipSlip 防止: 親ディレクトリ参照を含むエントリは弾く．
        let raw_name = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => {
                warn!(name = %entry.name(), "skipping zip entry with unsafe path");
                continue;
            }
        };
        if entry.is_dir() {
            continue;
        }
        // ファイル名のみを取り出して dest_dir 直下に展開する ( サブディレクトリは平坦化)．
        let basename = match raw_name.file_name() {
            Some(n) => n.to_owned(),
            None => continue,
        };

        let keep = if has_wtb {
            should_extract(&raw_name)
        } else {
            true
        };
        if !keep {
            continue;
        }

        let out_path = dest_dir.join(&basename);
        let mut out = File::create(&out_path)
            .with_context(|| format!("failed to create: {}", out_path.display()))?;
        std::io::copy(&mut entry, &mut out)
            .with_context(|| format!("failed to extract: {}", out_path.display()))?;
        extracted += 1;
    }
    Ok(extracted)
}

/// Keep only files with the `.wtb` / `.jou` / `.tou` extensions
/// (case-insensitive).
fn should_extract(path: &Path) -> bool {
    has_wtb_extension(path) || has_aux_extension(path)
}

fn has_wtb_extension(path: &Path) -> bool {
    matches_ext(path, "wtb")
}

fn has_aux_extension(path: &Path) -> bool {
    matches_ext(path, "jou") || matches_ext(path, "tou")
}

fn matches_ext(path: &Path, ext: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_url_replaces_year() {
        let url = expand_url("https://example.com/wth_{YEAR}.zip", 2023).unwrap();
        assert_eq!(url, "https://example.com/wth_2023.zip");
    }

    #[test]
    fn expand_url_pads_short_year_to_four_digits() {
        let url = expand_url("https://example.com/wth_{YEAR}.zip", 99).unwrap();
        assert_eq!(url, "https://example.com/wth_0099.zip");
    }

    #[test]
    fn expand_url_rejects_missing_placeholder() {
        let err = expand_url("https://example.com/wth.zip", 2023).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("{YEAR}"), "msg = {msg}");
    }

    #[test]
    fn parse_year_range_inclusive() {
        let (a, b) = parse_year_range("2020..2023").unwrap();
        assert_eq!(a, 2020);
        assert_eq!(b, 2023);
        // 両端含むはず
        let years: Vec<u32> = (a..=b).collect();
        assert_eq!(years, vec![2020, 2021, 2022, 2023]);
    }

    #[test]
    fn parse_year_range_single() {
        let (a, b) = parse_year_range("2023..2023").unwrap();
        assert_eq!(a, 2023);
        assert_eq!(b, 2023);
    }

    #[test]
    fn parse_year_range_rejects_inverted() {
        let err = parse_year_range("2023..2020").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("after end"), "msg = {msg}");
    }

    #[test]
    fn parse_year_range_rejects_garbage() {
        assert!(parse_year_range("abc").is_err());
        assert!(parse_year_range("2020").is_err());
        assert!(parse_year_range("2020-2023").is_err());
        assert!(parse_year_range("2020..xxx").is_err());
    }

    #[test]
    fn resolve_years_year_only() {
        let v = resolve_years(Some(2023), None).unwrap();
        assert_eq!(v, vec![2023]);
    }

    #[test]
    fn resolve_years_range() {
        let v = resolve_years(None, Some("2020..2022")).unwrap();
        assert_eq!(v, vec![2020, 2021, 2022]);
    }

    #[test]
    fn resolve_years_rejects_both() {
        let err = resolve_years(Some(2023), Some("2020..2023")).unwrap_err();
        assert!(format!("{err}").contains("mutually exclusive"));
    }

    #[test]
    fn resolve_years_rejects_neither() {
        let err = resolve_years(None, None).unwrap_err();
        assert!(format!("{err}").contains("--year"));
    }

    #[test]
    fn resolve_url_pattern_prefers_cli() {
        // 一時的に env をセット
        // 注意: 同プロセス内の他テストと干渉しないよう独立した env 名を使う
        // ここではグローバル env を使うので unsafe ブロックは無いが，
        // テストは serial に走るとは限らない．並行性安全のため env を直接いじらず，
        // CLI 引数優先のみを確認する．
        let s = resolve_url_pattern(Some("https://override.example/{YEAR}.zip"));
        assert_eq!(s, "https://override.example/{YEAR}.zip");
    }

    #[test]
    fn validate_url_pattern_ok() {
        validate_url_pattern("https://example.com/wth_{YEAR}.zip").unwrap();
    }

    #[test]
    fn validate_url_pattern_missing_placeholder() {
        let err = validate_url_pattern("https://example.com/wth.zip").unwrap_err();
        assert!(format!("{err}").contains("{YEAR}"));
    }

    #[test]
    fn print_list_contains_wthor_entry() {
        let mut buf = Vec::new();
        print_list(&mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("wthor"));
        assert!(s.contains("French Othello Federation"));
        assert!(s.contains("ffothello.org"));
        assert!(s.contains("docs/external-data.md"));
    }

    #[test]
    fn extension_helpers_case_insensitive() {
        assert!(has_wtb_extension(Path::new("WTH_2023.WTB")));
        assert!(has_wtb_extension(Path::new("wth.wtb")));
        assert!(!has_wtb_extension(Path::new("readme.txt")));
        assert!(has_aux_extension(Path::new("JOUEUR.JOU")));
        assert!(has_aux_extension(Path::new("tournoi.tou")));
        assert!(!has_aux_extension(Path::new("foo.zip")));
    }
}
