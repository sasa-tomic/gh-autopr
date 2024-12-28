use crate::tui::{render_message, App};
use ratatui::style::Color;
use ratatui::{backend::Backend, Terminal};
use std::process::Command;

pub fn git_ensure_in_repo(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()?;

    if !output.status.success() {
        app.add_log("ERROR", "Not in a git repository.");
        std::process::exit(1);
    }

    Ok(())
}

pub fn git_ensure_not_detached_head<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    branch_name: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    if branch_name == "HEAD" {
        app.add_log(
            "ERROR",
            "Detached HEAD state detected. Please check out a branch.",
        );
        render_message(
            terminal,
            "Error",
            "Detached HEAD state detected. Please check out a branch.",
            Color::Red,
        )?;
        std::process::exit(1);
    }
    Ok(())
}

pub fn git_cd_to_repo_root(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if output.status.success() {
        let repo_root = String::from_utf8(output.stdout)?.trim().to_string();
        std::env::set_current_dir(&repo_root)?;
        app.add_log(
            "INFO",
            format!("Changed directory to repo root: {}", repo_root),
        );
    } else {
        app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn git_diff_uncommitted(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--", ".", ":!*.lock"])
        .output()?;

    if !output.status.success() {
        app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
        return Err("Failed to get diff".into());
    }

    let diff_context = String::from_utf8(output.stdout)?.trim().to_string();

    if diff_context.is_empty() {
        let output = Command::new("git")
            .args(["diff", "--", ".", ":!*.lock"])
            .output()?;

        if !output.status.success() {
            app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
            return Err("Failed to get diff".into());
        }

        return Ok(String::from_utf8(output.stdout)?.trim().to_string());
    }

    Ok(diff_context)
}

pub fn git_diff_between_branches(
    app: &mut App,
    main_branch: &String,
    current_branch: &String,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args([
            "diff",
            &format!("{}...{}", main_branch, current_branch),
            "--",
            ":!*.lock",
        ])
        .output()?;

    if !output.status.success() {
        app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
        return Err("Failed to get diff between branches".into());
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

pub fn git_main_branch(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    let mut main_branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "origin/HEAD"])
        .output()?;

    if !main_branch_output.status.success() {
        app.add_log("INFO", "Setting origin HEAD automatically...");
        let output = Command::new("git")
            .args(["remote", "set-head", "origin", "--auto"])
            .output()?;

        if !output.status.success() {
            app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
            return Err("Failed to set origin HEAD".into());
        }

        main_branch_output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "origin/HEAD"])
            .output()?;

        if !main_branch_output.status.success() {
            app.add_error(String::from_utf8_lossy(&main_branch_output.stderr).to_string());
            return Err("Failed to determine main branch".into());
        }
    }

    let branch = String::from_utf8(main_branch_output.stdout)?
        .trim()
        .trim_start_matches("origin/")
        .to_string();
    app.add_log("INFO", format!("Determined main branch: {}", branch));
    Ok(branch)
}

pub fn git_current_branch(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;

    if !output.status.success() {
        app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
        return Err("Failed to get current branch".into());
    }

    let branch = String::from_utf8(output.stdout)?.trim().to_string();
    app.add_log("INFO", format!("Current branch: {}", branch));
    Ok(branch)
}

pub fn git_fetch_main(
    app: &mut App,
    current_branch: &String,
    main_branch: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    if current_branch == main_branch {
        let output = Command::new("git").args(["pull", "origin"]).output()?;
        if !output.status.success() {
            app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
            return Err("Failed to pull from origin".into());
        }
        app.add_log("INFO", "Pulled latest changes from origin");
    } else {
        let output = Command::new("git")
            .args([
                "fetch",
                "origin",
                format!("{}:{}", main_branch, main_branch).as_str(),
            ])
            .output()?;
        if !output.status.success() {
            app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
            return Err("Failed to fetch main branch".into());
        }
        app.add_log("INFO", format!("Fetched latest {} branch", main_branch));
    }

    Ok(())
}

pub fn git_stage_and_commit(
    app: &mut App,
    commit_title: &str,
    commit_details: &Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").args(["add", "."]).output()?;
    if !output.status.success() {
        app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
        return Err("Failed to stage changes".into());
    }
    app.add_log("INFO", "Staged all changes");

    let mut commit_message = commit_title.trim().to_string();
    if let Some(details) = commit_details {
        commit_message.push_str(&format!("\n\n{}", details.trim()));
    }

    let output = Command::new("git")
        .args(["commit", "-m", &commit_message])
        .output()?;
    if !output.status.success() {
        app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
        return Err("Failed to commit changes".into());
    }
    app.add_log("INFO", "Committed changes successfully");

    Ok(())
}

pub fn git_push_branch(app: &mut App, branch_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(["push", "origin", branch_name])
        .output()?;
    if !output.status.success() {
        app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
        return Err("Failed to push branch".into());
    }
    app.add_log("INFO", format!("Pushed branch {} to origin", branch_name));
    Ok(())
}

pub fn create_pull_request(
    app: &mut App,
    title: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("gh")
        .args(["pr", "create", "--title", title, "--body", body])
        .output()?;
    if !output.status.success() {
        app.add_error(String::from_utf8_lossy(&output.stderr).to_string());
        return Err("Failed to create pull request".into());
    }
    app.add_log("SUCCESS", "Pull request created successfully");
    Ok(())
}
