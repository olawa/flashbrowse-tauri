use crate::fs_commands::dirs_home;
use crate::models::{TabCompletionResult, TerminalOutput};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const COMMON_COMMANDS: &[&str] = &[
    "cd", "ls", "pwd", "mkdir", "rmdir", "cp", "mv", "rm", "touch", "cat", "less", "more", "head", "tail",
    "grep", "egrep", "fgrep", "find", "du", "df", "chmod", "chown", "ps", "top", "kill", "pkill", "killall",
    "open", "clear", "echo", "export", "source", "which", "where", "env", "history", "alias", "unalias",
    "git", "ssh", "scp", "rsync", "sftp", "curl", "wget", "tar", "gzip", "gunzip", "zip", "unzip",
    "python", "python3", "pip", "pip3", "conda", "mamba", "micromamba", "snakemake", "nextflow",
    "rsnap", "samtools", "bcftools", "bedtools", "tabix", "bgzip", "fastqc", "multiqc", "bwa", "bowtie2", "minimap2",
    "nano", "vim", "vi", "emacs", "code", "zsh", "bash", "sh", "brew", "cargo", "rustc", "swift", "swiftc",
    "make", "cmake", "docker", "singularity", "apptainer", "slurm", "sbatch", "squeue", "scancel",
];

#[tauri::command]
pub fn run_command(cmd: &str, cwd: &str) -> Result<TerminalOutput, String> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return Ok(TerminalOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            new_cwd: None,
        });
    }

    let working_dir = if cwd.is_empty() || cwd == "~" {
        dirs_home()
    } else if cwd.starts_with('~') {
        dirs_home().join(cwd.trim_start_matches("~/").trim_start_matches('~'))
    } else {
        PathBuf::from(cwd)
    };

    // 1. Handle "cd" command directly
    if trimmed == "cd" || trimmed == "cd ~" {
        let home = dirs_home().to_string_lossy().to_string();
        return Ok(TerminalOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
            new_cwd: Some(home),
        });
    } else if trimmed.starts_with("cd ") {
        let target_str = trimmed[3..].trim().trim_matches('"').trim_matches('\'');
        let target_path = if target_str == "~" || target_str.starts_with("~/") {
            dirs_home().join(target_str.trim_start_matches("~/").trim_start_matches('~'))
        } else if target_str.starts_with('/') {
            PathBuf::from(target_str)
        } else {
            working_dir.join(target_str)
        };

        if target_path.is_dir() {
            if let Ok(canonical) = target_path.canonicalize() {
                return Ok(TerminalOutput {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: 0,
                    new_cwd: Some(canonical.to_string_lossy().to_string()),
                });
            } else {
                return Ok(TerminalOutput {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: 0,
                    new_cwd: Some(target_path.to_string_lossy().to_string()),
                });
            }
        } else {
            return Ok(TerminalOutput {
                stdout: String::new(),
                stderr: format!("cd: no such file or directory: {}\n", target_str),
                exit_code: 1,
                new_cwd: None,
            });
        }
    }

    // 2. Handle generic shell execution
    #[cfg(not(target_os = "windows"))]
    let output_res = Command::new("sh")
        .arg("-c")
        .arg(trimmed)
        .current_dir(&working_dir)
        .output();

    #[cfg(target_os = "windows")]
    let output_res = Command::new("cmd")
        .args(["/c", trimmed])
        .current_dir(&working_dir)
        .output();

    match output_res {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let exit_code = out.status.code().unwrap_or(-1);

            Ok(TerminalOutput {
                stdout,
                stderr,
                exit_code,
                new_cwd: None,
            })
        }
        Err(e) => Err(format!("Failed to execute command: {}", e)),
    }
}

#[tauri::command]
pub fn tab_complete(input: &str, cwd: &str) -> Result<TabCompletionResult, String> {
    if input.is_empty() {
        return Ok(TabCompletionResult {
            completed_line: String::new(),
            suggestions: Vec::new(),
        });
    }

    let working_dir = if cwd.is_empty() || cwd == "~" {
        dirs_home()
    } else if cwd.starts_with('~') {
        dirs_home().join(cwd.trim_start_matches("~/").trim_start_matches('~'))
    } else {
        PathBuf::from(cwd)
    };

    let (prefix, token) = split_last_token(input);

    let is_first_word = prefix.trim().is_empty()
        || prefix.ends_with("| ")
        || prefix.ends_with("&& ")
        || prefix.ends_with("; ")
        || prefix.ends_with("|| ");

    // 1. First word command completion
    if is_first_word && !token.contains('/') && !token.starts_with('.') && !token.starts_with('~') {
        let token_lower = token.to_lowercase();
        let matches: Vec<String> = COMMON_COMMANDS
            .iter()
            .filter(|cmd| cmd.to_lowercase().starts_with(&token_lower))
            .map(|s| s.to_string())
            .collect();

        if matches.len() == 1 {
            return Ok(TabCompletionResult {
                completed_line: format!("{}{}{}", prefix, matches[0], " "),
                suggestions: Vec::new(),
            });
        } else if matches.len() > 1 {
            let lcp = longest_common_prefix(&matches);
            if lcp.len() > token.len() {
                return Ok(TabCompletionResult {
                    completed_line: format!("{}{}", prefix, lcp),
                    suggestions: Vec::new(),
                });
            } else {
                return Ok(TabCompletionResult {
                    completed_line: input.to_string(),
                    suggestions: matches,
                });
            }
        }
    }

    // 2. File / Directory Path Completion
    let unescaped_token = token.replace("\\ ", " ").replace("\\(", "(").replace("\\)", ")");
    let has_tilde = unescaped_token.starts_with('~');
    let resolved_token = if has_tilde {
        dirs_home().join(unescaped_token.trim_start_matches("~/").trim_start_matches('~'))
    } else {
        PathBuf::from(&unescaped_token)
    };

    let (parent_dir_path, partial_name, token_dir_prefix) = if unescaped_token.ends_with('/') {
        let parent = if has_tilde {
            resolved_token.clone()
        } else if unescaped_token.starts_with('/') {
            PathBuf::from(&unescaped_token)
        } else {
            working_dir.join(&unescaped_token)
        };
        (parent, String::new(), unescaped_token.clone())
    } else if let Some(last_slash_idx) = unescaped_token.rfind('/') {
        let dir_part = &unescaped_token[..=last_slash_idx];
        let name_part = &unescaped_token[last_slash_idx + 1..];
        let parent = if has_tilde {
            dirs_home().join(dir_part.trim_start_matches("~/").trim_start_matches('~'))
        } else if dir_part.starts_with('/') {
            PathBuf::from(dir_part)
        } else {
            working_dir.join(dir_part)
        };
        (parent, name_part.to_string(), dir_part.to_string())
    } else {
        (working_dir.clone(), unescaped_token.clone(), String::new())
    };

    if !parent_dir_path.exists() {
        return Ok(TabCompletionResult {
            completed_line: input.to_string(),
            suggestions: Vec::new(),
        });
    }

    let read_entries = fs::read_dir(&parent_dir_path).map_err(|e| e.to_string())?;
    let partial_lower = partial_name.to_lowercase();

    let mut matches: Vec<(String, bool)> = Vec::new();
    for entry in read_entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if partial_name.is_empty() {
            if !name.starts_with('.') {
                matches.push((name, is_dir));
            }
        } else if partial_name.starts_with('.') {
            if name.to_lowercase().starts_with(&partial_lower) {
                matches.push((name, is_dir));
            }
        } else {
            if !name.starts_with('.') && name.to_lowercase().starts_with(&partial_lower) {
                matches.push((name, is_dir));
            }
        }
    }

    matches.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    if matches.is_empty() {
        return Ok(TabCompletionResult {
            completed_line: input.to_string(),
            suggestions: Vec::new(),
        });
    }

    let match_names: Vec<String> = matches.iter().map(|m| m.0.clone()).collect();

    if matches.len() == 1 {
        let (matched_name, is_dir) = &matches[0];
        let suffix = if *is_dir { "/" } else { " " };
        let escaped_name = escape_shell_chars(matched_name);
        let completed = format!("{}{}{}{}", prefix, token_dir_prefix, escaped_name, suffix);

        Ok(TabCompletionResult {
            completed_line: completed,
            suggestions: Vec::new(),
        })
    } else {
        let lcp = longest_common_prefix(&match_names);
        if lcp.len() > partial_name.len() {
            let escaped_lcp = escape_shell_chars(&lcp);
            let completed = format!("{}{}{}", prefix, token_dir_prefix, escaped_lcp);
            Ok(TabCompletionResult {
                completed_line: completed,
                suggestions: Vec::new(),
            })
        } else {
            let suggestions: Vec<String> = matches
                .iter()
                .map(|(name, is_dir)| {
                    if *is_dir {
                        format!("{}/", name)
                    } else {
                        name.clone()
                    }
                })
                .collect();

            Ok(TabCompletionResult {
                completed_line: input.to_string(),
                suggestions,
            })
        }
    }
}

fn split_last_token(input: &str) -> (&str, &str) {
    let mut last_space = None;
    let mut is_escaped = false;

    for (idx, ch) in input.char_indices() {
        if ch == '\\' {
            is_escaped = !is_escaped;
        } else {
            if ch == ' ' && !is_escaped {
                last_space = Some(idx);
            }
            is_escaped = false;
        }
    }

    if let Some(space_idx) = last_space {
        (&input[..=space_idx], &input[space_idx + 1..])
    } else {
        ("", input)
    }
}

fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let first = &strings[0];
    let mut common_len = first.len();

    for s in &strings[1..] {
        let mut matching = 0;
        for (c1, c2) in first.chars().zip(s.chars()) {
            if c1.to_lowercase().to_string() == c2.to_lowercase().to_string() {
                matching += c1.len_utf8();
            } else {
                break;
            }
        }
        common_len = common_len.min(matching);
    }

    first[..common_len].to_string()
}

fn escape_shell_chars(str: &str) -> String {
    str.replace(' ', "\\ ")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace('&', "\\&")
        .replace(';', "\\;")
}
