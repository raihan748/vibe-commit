use clap::Parser;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use vibe_commit::cli::args::{Cli, Commands, OutputFormat};
use vibe_commit::planner::formatter::PlanFormatter;
use vibe_commit::plan_from_diff;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Plan {
            patch_file,
            format,
            max_commits,
            output,
        }) => {
            handle_plan(patch_file, format, max_commits, output);
        }
        Some(Commands::Apply {
            patch_file,
            dry_run,
            max_commits,
        }) => {
            handle_apply(patch_file, dry_run, max_commits);
        }
        None => {
            // Default plan mode
            handle_plan(cli.default_patch, cli.format, cli.max_commits, cli.output);
        }
    }
}

fn handle_plan(
    patch_file: Option<PathBuf>,
    format: OutputFormat,
    max_commits: Option<usize>,
    output: Option<PathBuf>,
) {
    let content = match read_patch_input(patch_file.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading patch input: {}", e);
            process::exit(1);
        }
    };

    let plan = match plan_from_diff(&content, max_commits) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error parsing diff: {}", e);
            process::exit(1);
        }
    };

    let formatted = match PlanFormatter::format(&plan, format, false) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error formatting plan: {}", e);
            process::exit(1);
        }
    };

    if let Some(out_path) = output {
        if let Err(e) = std::fs::write(&out_path, formatted) {
            eprintln!("Error writing output to {}: {}", out_path.display(), e);
            process::exit(1);
        }
        println!("Commit plan successfully written to {}", out_path.display());
    } else {
        print!("{}", formatted);
    }
}

fn handle_apply(
    patch_file: Option<PathBuf>,
    dry_run: bool,
    max_commits: Option<usize>,
) {
    let content = match read_patch_input(patch_file.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading patch input: {}", e);
            process::exit(1);
        }
    };

    let plan = match plan_from_diff(&content, max_commits) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error parsing diff: {}", e);
            process::exit(1);
        }
    };

    if dry_run {
        println!("[vibe-commit] DRY RUN: Verifying commit plan for {} commits...", plan.commits.len());
        let shell_preview = PlanFormatter::format_shell(&plan, true);
        println!("{}", shell_preview);
        return;
    }

    println!("[vibe-commit] Applying {} atomic conventional commits...", plan.commits.len());
    for commit in &plan.commits {
        println!("-> Staging and committing: {}", commit.title);
        for f in &commit.files {
            let status = Command::new("git")
                .arg("add")
                .arg(f)
                .status();
            match status {
                Ok(s) if s.success() => {}
                Ok(s) => eprintln!("Warning: git add {} returned exit code: {}", f, s),
                Err(e) => eprintln!("Warning: failed to execute git add {}: {}", f, e),
            }
        }

        let body = commit.body.join("\n");
        let mut cmd = Command::new("git");
        cmd.arg("commit").arg("-m").arg(&commit.title);
        if !body.is_empty() {
            cmd.arg("-m").arg(&body);
        }

        let commit_status = cmd.status();
        match commit_status {
            Ok(s) if s.success() => println!("   [OK] Committed: {}", commit.title),
            Ok(s) => eprintln!("   [WARN] Git commit exited with code: {}", s),
            Err(e) => eprintln!("   [ERR] Failed to execute git commit: {}", e),
        }
    }
    println!("[vibe-commit] All atomic commits successfully executed!");
}

fn read_patch_input(path_opt: Option<&Path>) -> io::Result<String> {
    if let Some(path) = path_opt {
        if path.as_os_str() != "-" {
            return std::fs::read_to_string(path);
        }
    }

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}
