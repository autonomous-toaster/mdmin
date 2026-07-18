//! mdmin benchmark suite.
//!
//! Measures token savings, monotonicity, structure preservation,
//! grammar strip coverage, and dictionary reversibility across
//! a corpus of real-world Markdown files.

#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use mdmin::{Config, GrammarLevel, Level, Minifier};

// ─── Corpus ─────────────────────────────────────────────────────────────────

fn discover_corpus() -> Vec<PathBuf> {
    // Use MDMIN_BENCH_CORPUS env var (comma-separated paths) or fall back
    // to relative paths from the crate root.
    let dirs: Vec<PathBuf> = if let Ok(corpus) = std::env::var("MDMIN_BENCH_CORPUS") {
        corpus.split(',').map(|s| PathBuf::from(s.trim())).collect()
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        vec![
            root.join("../skills"),
            root.join("../../anthropic/skills/skills"),
        ]
    };

    let mut files = Vec::new();
    for dir in &dirs {
        if dir.exists() {
            walk_dir(dir, &mut files);
        }
    }
    files
}

fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(&path, files);
            } else if path.extension().is_some_and(|e| e == "md") {
                files.push(path);
            }
        }
    }
}

// ─── Token counting ─────────────────────────────────────────────────────────

fn count_tokens(text: &str) -> usize {
    text.len() / 4
}

// ─── Structure extraction ────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
struct DocumentStructure {
    headings: Vec<String>,
    list_items: Vec<String>,
    table_rows: Vec<Vec<String>>,
}

/// Normalize a heading for comparison: lowercase, strip trailing colon.
fn normalize_heading(h: &str) -> String {
    h.trim()
        .trim_start_matches('#')
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_lowercase()
}

fn extract_structure(text: &str) -> DocumentStructure {
    let mut struct_ = DocumentStructure::default();

    for line in text.lines() {
        let trimmed = line.trim();

        // Headings: ## Title, # Title, or normalized "title:"
        if trimmed.starts_with('#') {
            let h = normalize_heading(trimmed);
            if !h.is_empty() {
                struct_.headings.push(h);
            }
        } else if let Some(h) = trimmed.strip_suffix(':') {
            let h = h.trim();
            if !h.is_empty() && !h.contains(' ') && h.chars().all(|c| c.is_alphanumeric() || c == '-') {
                struct_.headings.push(h.to_lowercase());
            }
        }

        // List items: - text or * text
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            struct_.list_items.push(item.to_string());
        }

        // Table rows: | cell1 | cell2 |
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            let cells: Vec<String> = trimmed
                .split('|')
                .skip(1)
                .filter(|c| !c.is_empty() && c.trim() != "-")
                .map(|c| c.trim().to_string())
                .collect();
            if !cells.is_empty() {
                struct_.table_rows.push(cells);
            }
        }
    }

    struct_
}

// ─── Grammar strip coverage ─────────────────────────────────────────────────

fn is_approved_removal(word: &str) -> bool {
    let lower = word.to_lowercase();

    // Frequency table words
    let freq_words: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "however", "furthermore",
        "additionally", "moreover", "nevertheless", "nonetheless",
        "is", "are", "was", "were", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "can", "could",
        "just", "really", "very", "quite", "simply", "actually", "basically",
        "essentially", "generally", "extremely", "incredibly", "absolutely",
        "totally", "completely", "utterly", "highly", "particularly",
        "especially", "truly", "also", "even", "still", "already",
        "always", "often", "usually", "typically", "currently",
        "rather", "pretty", "somewhat", "somehow", "anyway",
        "albeit", "whereas",
        "thus", "hence", "therefore", "consequently", "accordingly",
        "meanwhile", "likewise", "similarly", "conversely",
        "instead", "otherwise", "namely", "specifically",
        "perhaps", "possibly", "maybe",
        "it", "its", "this", "that", "these", "those", "we", "our",
        "you", "your", "they", "their", "he", "she", "his", "her",
        "itself", "themselves", "some", "any", "each", "every", "all",
        "both", "no", "other", "such",
    ];

    if freq_words.contains(&lower.as_str()) {
        return true;
    }

    // Replacement targets (words that get replaced, not removed)
    let replacements: &[&str] = &[
        "implement", "implements", "implemented", "implementing", "implementation",
        "implementations", "reimplement", "reimplementing", "reimplemented",
        "demonstrate", "demonstrates", "demonstrated", "demonstration",
        "sufficient", "insufficient", "additional", "subsequent", "preceding",
        "facilitate", "endeavor", "utilize", "utilizes", "utilized",
        "configuration", "documentation", "authentication", "authorization",
        "repository", "directory", "identifier", "initialize",
        "description", "environment", "operations", "variables",
        "dependencies", "resources", "processing", "architecture",
        "management", "functions", "parameter", "references",
        "version", "versions", "project", "projects",
        "context", "contexts", "response", "responses",
        "integration", "integrations", "structure", "structures",
        "command", "commands", "message", "messages",
        "session", "sessions", "default", "defaults",
        "install", "installed", "installation",
        "specific", "options", "available",
        "multiple", "service", "services",
        "pattern", "patterns",
        "example", "examples",
    ];

    if replacements.contains(&lower.as_str()) {
        return true; // These are replaced, not removed — safe
    }

    // Protected words (should never be removed)
    let protected: &[&str] = &[
        "after", "before", "during", "until", "while", "since", "once",
        "above", "below", "beneath", "beside", "between", "beyond",
        "across", "among", "around", "behind", "against", "along",
        "inside", "outside", "over", "under", "upon", "via",
        "through", "into", "onto", "within", "without", "about",
        "despite", "except",
    ];

    if protected.contains(&lower.as_str()) {
        return false;
    }

    false
}

// ─── Dictionary reversibility ───────────────────────────────────────────────

fn extract_dict(text: &str) -> HashMap<String, String> {
    let mut dict = HashMap::new();
    let mut in_dict = false;

    for line in text.lines() {
        if line.starts_with("@dict:") {
            in_dict = true;
            continue;
        }
        if in_dict {
            if line.starts_with("  @") {
                if let Some((key, value)) = line.trim().split_once(": ") {
                    dict.insert(key.to_string(), value.to_string());
                }
            } else {
                in_dict = false;
            }
        }
    }

    dict
}

fn expand_dict(text: &str, dict: &HashMap<String, String>) -> String {
    // Split into header and body to avoid expanding @N in the @dict header
    let mut header = Vec::new();
    let mut body = Vec::new();
    let mut in_dict = false;

    for line in text.lines() {
        if line.starts_with("@dict:") {
            in_dict = true;
            header.push(line);
        } else if in_dict && line.starts_with("  @") {
            header.push(line);
        } else {
            if in_dict {
                in_dict = false;
            }
            body.push(line);
        }
    }

    // Expand only the body
    let mut body_text = body.join("\n");
    let mut keys: Vec<&String> = dict.keys().collect();
    keys.sort_by(|a, b| b.len().cmp(&a.len()));

    for key in keys {
        if let Some(value) = dict.get(key) {
            body_text = body_text.replace(key.as_str(), value);
        }
    }

    // Rejoin header and expanded body
    if header.is_empty() {
        body_text
    } else {
        format!("{}\n{}\n", header.join("\n"), body_text)
    }
}

/// Remove @dict header from text for comparison.
fn strip_dict_header(text: &str) -> String {
    text.lines()
        .filter(|l| !l.starts_with("@dict:") && !l.starts_with("  @"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ─── Benchmark runner ────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
struct BenchmarkReport {
    corpus_files: usize,
    corpus_bytes: usize,
    token_savings: HashMap<String, TokenSavings>,
    monotonicity_violations: Vec<String>,
    structure_preservation: HashMap<String, StructureStats>,
    grammar_coverage: GrammarCoverage,
    dictionary_reversibility: DictReversibility,
    duration_ms: u64,
}

#[derive(Debug, Clone)]
struct TokenSavings {
    tokens: usize,
    bytes: usize,
    savings_pct: f64,
}

#[derive(Debug, Clone, Default)]
struct StructureStats {
    headings_total: usize,
    headings_match: usize,
    lists_total: usize,
    lists_match: usize,
    tables_total: usize,
    tables_match: usize,
}

#[derive(Debug, Default, Clone)]
struct GrammarCoverage {
    total_removed: usize,
    in_frequency_table: usize,
    replaced: usize,
    protected_removed: usize,
    unexpected: usize,
    unexpected_words: Vec<String>,
}

#[derive(Debug, Default, Clone)]
struct DictReversibility {
    files_with_dict: usize,
    fully_reversible: usize,
    irreversibility_issues: Vec<String>,
}

fn run_benchmark() -> BenchmarkReport {
    let start = Instant::now();
    let mut report = BenchmarkReport::default();

    let files = discover_corpus();
    report.corpus_files = files.len();

    // Level-only configs (no features) for monotonicity and structure
    let level_configs: Vec<(String, Config)> = (0..5)
        .map(|l| {
            let level = match l {
                0 => Level::Off,
                1 => Level::Light,
                2 => Level::Medium,
                3 => Level::Structured,
                _ => Level::Ultra,
            };
            (format!("L{l}"), Config::new(level))
        })
        .collect();

    // Feature configs for token savings breakdown (skip bare levels — already in level_configs)
    let feature_configs: Vec<(String, Config)> = {
        let mut cfgs = Vec::new();
        for (level, level_name) in &[(Level::Medium, "L2"), (Level::Structured, "L3"), (Level::Ultra, "L4")] {
            // Grammar only
            let mut c = Config::new(*level);
            c.grammar_strip = Some(GrammarLevel::Medium);
            cfgs.push((format!("{}_g", level_name), c));
            // Dictionary only
            let mut c = Config::new(*level);
            c.dictionary = true;
            cfgs.push((format!("{}_d", level_name), c));
            // Both
            let mut c = Config::new(*level);
            c.grammar_strip = Some(GrammarLevel::Medium);
            c.dictionary = true;
            cfgs.push((format!("{}_gd", level_name), c));
        }
        cfgs
    };

    for file in &files {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if content.len() < 50 {
            continue;
        }

        report.corpus_bytes += content.len();
        let orig_struct = extract_structure(&content);

        // ── Level configs (monotonicity + structure) ──
        let mut level_sizes: Vec<(u8, usize)> = Vec::new();
        let mut l2_output: Option<String> = None;

        for (name, config) in &level_configs {
            let mut minifier = match Minifier::new(config) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let result = match minifier.minify(&content) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let bytes = result.output.len();
            let tokens = count_tokens(&result.output);

            // Token savings
            let entry = report
                .token_savings
                .entry(name.clone())
                .or_insert_with(|| TokenSavings {
                    tokens: 0,
                    bytes: 0,
                    savings_pct: 0.0,
                });
            entry.tokens += tokens;
            entry.bytes += bytes;

            // Track for monotonicity
            let level_num = name[1..].parse::<u8>().unwrap_or(0);
            level_sizes.push((level_num, bytes));

            // Save L2 output for dictionary reversibility comparison
            if name == "L2" {
                l2_output = Some(result.output.clone());
            }

            // Structure preservation
            if level_num >= 1 {
                let comp_struct = extract_structure(&result.output);
                let stats = report
                    .structure_preservation
                    .entry(name.clone())
                    .or_insert_with(StructureStats::default);

                // Headings: compare normalized
                for h in &orig_struct.headings {
                    stats.headings_total += 1;
                    if comp_struct.headings.contains(h) {
                        stats.headings_match += 1;
                    }
                }

                // List items
                for item in &orig_struct.list_items {
                    stats.lists_total += 1;
                    if comp_struct.list_items.contains(item) {
                        stats.lists_match += 1;
                    }
                }

                // Table rows
                for row in &orig_struct.table_rows {
                    stats.tables_total += 1;
                    for crow in &comp_struct.table_rows {
                        if row == crow {
                            stats.tables_match += 1;
                            break;
                        }
                    }
                }
            }
        }

        // Monotonicity check
        level_sizes.sort_by_key(|(level, _)| *level);
        for i in 1..level_sizes.len() {
            if level_sizes[i].1 > level_sizes[i - 1].1 {
                report.monotonicity_violations.push(format!(
                    "{}: L{} ({}) > L{} ({})",
                    file.display(),
                    level_sizes[i].0,
                    level_sizes[i].1,
                    level_sizes[i - 1].0,
                    level_sizes[i - 1].1
                ));
            }
        }

        // ── Feature configs (grammar + dictionary) ──
        for (name, config) in &feature_configs {
            let mut minifier = match Minifier::new(config) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let result = match minifier.minify(&content) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let tokens = count_tokens(&result.output);
            let bytes = result.output.len();

            // Token savings (skip bare levels — already counted in level_configs)
            let is_bare = name == "L2" || name == "L3" || name == "L4";
            if !is_bare {
                let entry = report
                    .token_savings
                    .entry(name.to_string())
                    .or_insert_with(|| TokenSavings {
                        tokens: 0,
                        bytes: 0,
                        savings_pct: 0.0,
                    });
                entry.tokens += tokens;
                entry.bytes += bytes;
            }

            // Grammar strip coverage (only once per file, on L2_g)
            if name == "L2_g" {
                let no_g_config = Config::new(Level::Medium);
                let mut no_g_minifier = Minifier::new(&no_g_config).unwrap();
                let no_g_result = no_g_minifier.minify(&content).unwrap();

                let no_g_words: Vec<&str> = no_g_result
                    .output
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| !w.is_empty())
                    .collect();
                let g_words: Vec<&str> = result
                    .output
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| !w.is_empty())
                    .collect();

                for word in &no_g_words {
                    if !g_words.contains(word) && word.len() > 1 {
                        report.grammar_coverage.total_removed += 1;
                        if is_approved_removal(word) {
                            report.grammar_coverage.in_frequency_table += 1;
                        } else if word.chars().all(|c| c.is_uppercase()) && word.len() > 1 {
                            report.grammar_coverage.protected_removed += 1;
                            report
                                .grammar_coverage
                                .unexpected_words
                                .push(format!("RFC2119: {word}"));
                        } else {
                            // Check if it's a replacement target
                            let lower = word.to_lowercase();
                            let replacement_targets: &[&str] = &[
                                "implement", "implements", "implemented", "implementing",
                                "implementation", "implementations", "reimplement",
                                "reimplementing", "reimplemented",
                                "demonstrate", "demonstrates", "demonstrated",
                                "sufficient", "insufficient", "additional",
                                "subsequent", "preceding", "facilitate", "endeavor",
                                "utilize", "utilizes", "utilized",
                            ];
                            if replacement_targets.contains(&lower.as_str()) {
                                report.grammar_coverage.replaced += 1;
                            } else {
                                report.grammar_coverage.unexpected += 1;
                                report
                                    .grammar_coverage
                                    .unexpected_words
                                    .push(format!("unexpected: {word}"));
                            }
                        }
                    }
                }
            }

            // Dictionary reversibility (only once per file, on L2_d)
            if name == "L2_d" {
                let dict = extract_dict(&result.output);
                if !dict.is_empty() {
                    report.dictionary_reversibility.files_with_dict += 1;
                    let expanded = expand_dict(&result.output, &dict);
                    let expanded_clean = strip_dict_header(&expanded);
                    // Compare against L2 output (pre-dictionary)
                    if let Some(ref l2) = l2_output {
                        if expanded_clean.trim() == l2.trim() {
                            report.dictionary_reversibility.fully_reversible += 1;
                        } else {
                            report
                                .dictionary_reversibility
                                .irreversibility_issues
                                .push(format!("{}: expansion mismatch", file.display()));
                        }
                    }
                }
            }
        }
    }

    // Compute savings percentages (relative to L0)
    let _l0_tokens = report
        .token_savings
        .get("L0")
        .map(|s| s.tokens)
        .unwrap_or(1);
    let l0_bytes = report
        .token_savings
        .get("L0")
        .map(|s| s.bytes)
        .unwrap_or(1);
    for savings in report.token_savings.values_mut() {
        savings.savings_pct =
            (1.0 - savings.bytes as f64 / l0_bytes as f64) * 100.0;
    }

    report.duration_ms = start.elapsed().as_millis() as u64;
    report
}

// ─── Output ─────────────────────────────────────────────────────────────────

fn print_human(report: &BenchmarkReport) {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║           mdmin Benchmark Report                ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("Corpus: {} files ({} bytes)", report.corpus_files, report.corpus_bytes);
    println!("Duration: {} ms", report.duration_ms);
    println!();

    // Token savings
    println!("── Token Savings ──");
    let mut configs: Vec<(&String, &TokenSavings)> = report.token_savings.iter().collect();
    configs.sort_by(|a, b| a.0.cmp(b.0));
    for (name, savings) in &configs {
        println!(
            "  {:<6} {:>8} tokens {:>8} bytes ({:>5.1}%)",
            name, savings.tokens, savings.bytes, savings.savings_pct
        );
    }
    println!();

    // Monotonicity
    println!("── Monotonicity ──");
    if report.monotonicity_violations.is_empty() {
        println!("  ✅ No violations — L0 < L1 < L2 < L3 < L4 on all files");
    } else {
        for v in &report.monotonicity_violations {
            println!("  ❌ {v}");
        }
    }
    println!();

    // Structure preservation
    println!("── Structure Preservation ──");
    let mut structs: Vec<(&String, &StructureStats)> =
        report.structure_preservation.iter().collect();
    structs.sort_by(|a, b| a.0.cmp(b.0));
    for (name, stats) in &structs {
        let h_pct = if stats.headings_total > 0 {
            stats.headings_match as f64 / stats.headings_total as f64 * 100.0
        } else {
            100.0
        };
        let l_pct = if stats.lists_total > 0 {
            stats.lists_match as f64 / stats.lists_total as f64 * 100.0
        } else {
            100.0
        };
        let t_pct = if stats.tables_total > 0 {
            stats.tables_match as f64 / stats.tables_total as f64 * 100.0
        } else {
            100.0
        };
        println!(
            "  {:<6} headings: {:>5.1}%  lists: {:>5.1}%  tables: {:>5.1}%",
            name, h_pct, l_pct, t_pct
        );
    }
    println!();

    // Grammar coverage
    println!("── Grammar Strip Coverage ──");
    println!("  Total words removed:     {}", report.grammar_coverage.total_removed);
    println!(
        "  In frequency table:     {}",
        report.grammar_coverage.in_frequency_table
    );
    println!("  Replaced (safe):         {}", report.grammar_coverage.replaced);
    println!(
        "  Protected removed:      {}",
        report.grammar_coverage.protected_removed
    );
    println!("  Unexpected:              {}", report.grammar_coverage.unexpected);
    if !report.grammar_coverage.unexpected_words.is_empty() {
        println!("  Unexpected words (first 20):");
        for w in report.grammar_coverage.unexpected_words.iter().take(20) {
            println!("    - {w}");
        }
    }
    println!();

    // Dictionary reversibility
    println!("── Dictionary Reversibility ──");
    println!(
        "  Files with dict:        {}",
        report.dictionary_reversibility.files_with_dict
    );
    println!(
        "  Fully reversible:       {}",
        report.dictionary_reversibility.fully_reversible
    );
    if !report.dictionary_reversibility.irreversibility_issues.is_empty() {
        println!(
            "  Issues: {} files",
            report.dictionary_reversibility.irreversibility_issues.len()
        );
    }
    println!();
}

fn print_json(report: &BenchmarkReport) {
    println!("{{");
    println!(
        "  \"corpus\": {{ \"files\": {}, \"bytes\": {} }},",
        report.corpus_files, report.corpus_bytes
    );
    println!("  \"duration_ms\": {},", report.duration_ms);
    println!("  \"monotonicity_violations\": {},", report.monotonicity_violations.len());
    println!(
        "  \"grammar_coverage\": {{ \"total_removed\": {}, \"unexpected\": {} }},",
        report.grammar_coverage.total_removed, report.grammar_coverage.unexpected
    );
    println!(
        "  \"dictionary_reversibility\": {{ \"files_with_dict\": {}, \"fully_reversible\": {} }}",
        report.dictionary_reversibility.files_with_dict,
        report.dictionary_reversibility.fully_reversible
    );
    println!("}}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args.contains(&"--json".to_string());

    let report = run_benchmark();

    if json_mode {
        print_json(&report);
    } else {
        print_human(&report);
    }
}
