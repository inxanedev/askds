use std::process::Command;

/// Upper bound on how much history is sent to the model. DeepSeek models have
/// large context windows, but a terminal scrollback can be arbitrarily huge.
const MAX_HISTORY_CHARS: usize = 120_000;

/// Capture the visible terminal session, including command output.
///
/// The terminal's own scrollback is read. That means content removed by
/// `clear` (which clears the scrollback buffer in all supported terminals) is
/// absent automatically.
///
/// Returns `Err` with a human readable explanation when the terminal does not
/// expose its scrollback.
pub fn capture() -> Result<String, String> {
    let raw = if std::env::var_os("TMUX").is_some() {
        tmux()?
    } else if std::env::var_os("KITTY_LISTEN_ON").is_some() {
        kitty()?
    } else if std::env::var_os("WEZTERM_PANE").is_some()
        || std::env::var("TERM_PROGRAM")
            .map(|v| v.eq_ignore_ascii_case("wezterm"))
            .unwrap_or(false)
    {
        wezterm()?
    } else if std::env::var_os("KONSOLE_DBUS_SESSION").is_some()
        && std::env::var_os("KONSOLE_DBUS_SERVICE").is_some()
    {
        konsole()?
    } else {
        return Err(
            "this terminal does not expose its scrollback (supported: Konsole, tmux, kitty, WezTerm)"
                .to_string(),
        );
    };

    let cleaned = strip_invocation(&raw);
    if cleaned.trim().is_empty() {
        return Ok(String::new());
    }
    Ok(truncate_start(&cleaned, MAX_HISTORY_CHARS))
}

fn konsole() -> Result<String, String> {
    let service =
        std::env::var("KONSOLE_DBUS_SERVICE").map_err(|_| "missing KONSOLE_DBUS_SERVICE")?;
    let session =
        std::env::var("KONSOLE_DBUS_SESSION").map_err(|_| "missing KONSOLE_DBUS_SESSION")?;

    // Konsole's D-Bus method `getAllDisplayedText(true)` returns the current
    // scrollback plus screen, with trailing empty lines removed.
    for tool in ["qdbus6", "qdbus"] {
        let output = match Command::new(tool)
            .arg(&service)
            .arg(&session)
            .arg("org.kde.konsole.Session.getAllDisplayedText")
            .arg("true")
            .output()
        {
            Ok(output) => output,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("failed to run {tool}: {err}")),
        };

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{tool} could not read the Konsole scrollback")
        } else {
            format!("{tool} could not read the Konsole scrollback: {stderr}")
        });
    }

    Err("could not find qdbus6 or qdbus to read Konsole's scrollback".to_string())
}

fn tmux() -> Result<String, String> {
    let mut cmd = Command::new("tmux");
    cmd.args(["capture-pane", "-p", "-S", "-"]);
    if let Ok(pane) = std::env::var("TMUX_PANE") {
        cmd.args(["-t", &pane]);
    }

    let output = cmd
        .output()
        .map_err(|err| format!("failed to run tmux: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "tmux capture-pane failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn kitty() -> Result<String, String> {
    let output = Command::new("kitty")
        .args(["@", "get-text", "--extent", "all"])
        .output()
        .map_err(|err| format!("failed to run kitty: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "kitty get-text failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn wezterm() -> Result<String, String> {
    let output = Command::new("wezterm")
        .args(["cli", "get-text"])
        .output()
        .map_err(|err| format!("failed to run wezterm: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "wezterm get-text failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The last non-empty line is the `askds ...` invocation itself (the command
/// has not produced output yet). Remove it so the model does not see the
/// question twice.
fn strip_invocation(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();

    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    lines.pop();

    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

/// Keep the end (most recent output) and cut on a UTF-8 boundary.
fn truncate_start(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let total = text.chars().count();
    let skip = total - max_chars;
    let start = text
        .char_indices()
        .nth(skip)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len());
    format!("[earlier terminal output truncated]\n{}", &text[start..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_the_invocation_line() {
        let text = "user@host ~> echo hello\nhello\nuser@host ~> askds what is this\n";
        assert_eq!(strip_invocation(text), "user@host ~> echo hello\nhello");
    }

    #[test]
    fn removes_wrapping_blank_lines() {
        let text = "a\nb\naskds question\n\n\n";
        assert_eq!(strip_invocation(text), "a\nb");
    }

    #[test]
    fn handles_an_empty_screen() {
        assert_eq!(strip_invocation("^[[H^[[2J"), "");
    }

    #[test]
    fn truncates_from_the_start() {
        let text = "0123456789";
        assert_eq!(
            truncate_start(text, 5),
            "[earlier terminal output truncated]\n56789"
        );
    }
}
