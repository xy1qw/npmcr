use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io};

use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use serde_json::Value;

const EXCLUDE_DIRS: [&str; 1] = ["node_modules"];
const MAX_DEPTH: u32 = 4;

struct NpmCommand {
    name: String,
    path: String,
    command: String,
}

impl ToString for NpmCommand {
    fn to_string(&self) -> String {
        let display_path = if self.path == "." { "root" } else { &self.path };
        format!(
            "{} {} {}",
            style(format!("[{}]", display_path)).dim(),
            style(&self.name).green(),
            style(format!("{{ {} }}", &self.command)).dim(),
        )
    }
}

fn find_package_json_files(dir: &Path, level: u32) -> io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();

    if level > MAX_DEPTH {
        return Ok(result);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && !is_excluded_dir(&path) {
            result.extend(find_package_json_files(&path, level + 1)?);
        } else if path.is_file() && path.file_name() == Some("package.json".as_ref()) {
            result.push(path);
        }
    }

    Ok(result)
}

fn is_excluded_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| EXCLUDE_DIRS.contains(&name))
        .unwrap_or(false)
}

fn get_npm_scripts(file: &Path) -> io::Result<Vec<NpmCommand>> {
    let content = fs::read_to_string(file)?;
    let json: Value = serde_json::from_str(&content)?;

    let path = file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_string_lossy()
        .to_string();

    let scripts = json
        .get("scripts")
        .and_then(|s| s.as_object())
        .map(|scripts_map| {
            scripts_map
                .iter()
                .filter_map(|(name, command)| {
                    command.as_str().map(|cmd| (name.clone(), cmd.to_string()))
                })
                .map(|(name, command)| NpmCommand {
                    name,
                    command,
                    path: path.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(scripts)
}

fn main() -> io::Result<()> {
    let mut all_commands = Vec::new();

    for file in find_package_json_files(Path::new("."), 1)? {
        all_commands.extend(get_npm_scripts(&file)?);
    }

    if all_commands.is_empty() {
        println!("No commands found.");
        return Ok(());
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a command to execute:")
        .items(&all_commands)
        .interact_opt()
        .unwrap();

    if let Some(s) = selection {
        let selected_command = &all_commands[s];

        return Command::new("sh")
            .arg("-c")
            .arg(format!("npm run {}", selected_command.name))
            .current_dir(&selected_command.path)
            .status()
            .map(|status| {
                if status.success() {
                    println!("Command executed successfully.");
                } else {
                    eprintln!("Error executing command.");
                }
            });
    }

    Ok(())
}
