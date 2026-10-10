//! Open a `file:line` location in the user's editor.

use std::path::Path;

/// Whether the editor is a GUI app (`code`, `zed`) that should be launched
/// detached, or a terminal editor run in the foreground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Zed,
    VsCode,
    Neovim,
    Vim,
    Helix,
    /// Anything else, including `$EDITOR` values with arguments baked in.
    Custom,
}

impl Kind {
    fn is_gui(&self) -> bool {
        matches!(self, Kind::Zed | Kind::VsCode)
    }

    fn display_name(&self) -> &'static str {
        match self {
            Kind::Zed => "Zed",
            Kind::VsCode => "VS Code",
            Kind::Neovim => "Neovim",
            Kind::Vim => "Vim",
            Kind::Helix => "Helix",
            Kind::Custom => "Custom",
        }
    }
}

fn display_name(bin: &str, kind: Kind) -> String {
    match bin {
        "cursor" => "Cursor".to_string(),
        "windsurf" => "Windsurf".to_string(),
        "code-insiders" => "VS Code Insiders".to_string(),
        "vscodium" => "VSCodium".to_string(),
        _ => kind.display_name().to_string(),
    }
}

/// A resolved editor: the binary to run plus how it should be launched.
#[derive(Debug, Clone)]
pub struct Editor {
    name: String,
    program: String,
    kind: Kind,
    gui: bool,
}

impl Editor {
    fn new(bin: &str, kind: Kind) -> Self {
        let gui = kind.is_gui();
        Editor {
            name: display_name(bin, kind),
            program: bin.to_string(),
            kind,
            gui,
        }
    }

    /// Resolve which editor to use, most specific first:
    ///
    /// 1. `TRAK_EDITOR` — an explicit per-project override.
    /// 2. The editor whose terminal we are running in (via `$ZED_TERM`,
    ///    `$NVIM`, `$TERM_PROGRAM`, or by walking the parent process tree).
    /// 3. `$VISUAL` / `$EDITOR`.
    /// 4. Any installed editor CLI, roughly the order the project suggests.
    pub fn detect() -> Option<Editor> {
        if let Some(editor) = explicit_override() {
            return Some(editor);
        }
        if let Some(editor) = surrounding_editor() {
            return Some(editor);
        }
        if let Some(editor) = env_fallback() {
            return Some(editor);
        }
        for bin in ["zed", "code", "cursor", "nvim", "vim", "hx"] {
            if command_exists(bin) {
                return Some(Editor::new(bin, kind_for(bin)));
            }
        }
        None
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Open `file` at `line`, preferring the running instance of the editor.
    pub fn open(&self, file: &str, line: usize) -> std::io::Result<()> {
        let (program, args) = self.command(file, line);
        if self.gui {
            std::process::Command::new(&program)
                .args(&args)
                .spawn()
                .map(|_| ())
        } else {
            std::process::Command::new(&program)
                .args(&args)
                .status()
                .map(|_| ())
        }
    }

    fn command(&self, file: &str, line: usize) -> (String, Vec<String>) {
        let loc = format!("{file}:{line}");
        match self.kind {
            // `--existing` reuses the currently open Zed window.
            Kind::Zed => (self.program.clone(), vec!["--existing".into(), loc]),
            // `-r` reuses a window, `-g` jumps to the line.
            Kind::VsCode => (self.program.clone(), vec!["-r".into(), "-g".into(), loc]),
            // If Neovim is already running, ask that instance to open the file.
            Kind::Neovim => {
                if let Ok(server) = std::env::var("NVIM") {
                    (
                        self.program.clone(),
                        vec![
                            "--server".into(),
                            server,
                            "--remote".into(),
                            format!("+{line}"),
                            file.into(),
                        ],
                    )
                } else {
                    (self.program.clone(), vec![format!("+{line}"), file.into()])
                }
            }
            Kind::Vim | Kind::Helix => {
                (self.program.clone(), vec![format!("+{line}"), file.into()])
            }
            Kind::Custom => {
                let mut parts = self.program.split_whitespace();
                let program = parts.next().unwrap_or("vi").to_string();
                let mut args: Vec<String> = parts.map(String::from).collect();
                if self.gui {
                    args.push(loc);
                } else {
                    args.push(format!("+{line}"));
                    args.push(file.to_string());
                }
                (program, args)
            }
        }
    }
}

/// `TRAK_EDITOR` is the only thing that overrides detection: it may be a bare
/// binary or a full command line such as `zed --wait` or `code -r`. A value
/// with spaces cannot be a plain program, so it falls back to the generic
/// `Custom` path which preserves the arguments.
fn explicit_override() -> Option<Editor> {
    from_env_value(std::env::var("TRAK_EDITOR").ok()?)
}

/// Turn a raw `$TRAK_EDITOR` / `$VISUAL` / `$EDITOR` value into an `Editor`.
fn from_env_value(program: String) -> Option<Editor> {
    let program = program.trim();
    if program.is_empty() {
        return None;
    }
    let bin = Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program);
    if program.contains(char::is_whitespace) {
        return Some(Editor {
            name: bin.to_string(),
            program: program.to_string(),
            kind: Kind::Custom,
            gui: false,
        });
    }
    Some(Editor::new(bin, kind_for(bin)))
}

/// Discover which editor we are running inside, cheaply from the environment
/// first and then by walking up the parent process tree.
fn surrounding_editor() -> Option<Editor> {
    // Zed marks every terminal it spawns.
    if std::env::var("ZED_TERM").is_ok() && command_exists("zed") {
        return Some(Editor::new("zed", Kind::Zed));
    }
    // Neovim exposes its attached server socket.
    if (std::env::var("NVIM").is_ok() || std::env::var("NVIM_LISTEN_ADDRESS").is_ok())
        && command_exists("nvim")
    {
        return Some(Editor::new("nvim", Kind::Neovim));
    }
    // VS Code family sets TERM_PROGRAM. Novel versions of Zed do too.
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        let editor = match term.as_str() {
            "vscode" => {
                if command_exists("code") {
                    Some(Editor::new("code", Kind::VsCode))
                } else {
                    None
                }
            }
            "cursor" => {
                if command_exists("cursor") {
                    Some(Editor::new("cursor", Kind::VsCode))
                } else {
                    None
                }
            }
            "zed" => {
                if command_exists("zed") {
                    Some(Editor::new("zed", Kind::Zed))
                } else {
                    None
                }
            }
            _ => None,
        };
        if editor.is_some() {
            return editor;
        }
    }
    current_editor_from_processes()
}

/// Walk up the process tree looking for a running editor. `trak` is usually
/// launched from a shell whose parents are the editor itself (its terminal,
/// its task runner, etc.), so a few levels are enough.
fn current_editor_from_processes() -> Option<Editor> {
    #[cfg(unix)]
    {
        fn parent_of(pid: i32) -> Option<i32> {
            let output = std::process::Command::new("ps")
                .args(["-o", "ppid=", "-p"])
                .arg(pid.to_string())
                .output()
                .ok()?;
            String::from_utf8_lossy(&output.stdout).trim().parse().ok()
        }

        fn comm_of(pid: i32) -> Option<String> {
            let output = std::process::Command::new("ps")
                .args(["-o", "comm=", "-p"])
                .arg(pid.to_string())
                .output()
                .ok()?;
            let comm = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_lowercase();
            if comm.is_empty() {
                return None;
            }
            Some(comm)
        }

        fn editor_from_comm(comm: &str) -> Option<Editor> {
            let base = Path::new(comm)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(comm)
                .to_lowercase();
            if base.contains("zed") {
                return command_exists("zed").then(|| Editor::new("zed", Kind::Zed));
            }
            if base.contains("cursor") {
                return command_exists("cursor").then(|| Editor::new("cursor", Kind::VsCode));
            }
            if base.contains("windsurf") {
                return command_exists("windsurf").then(|| Editor::new("windsurf", Kind::VsCode));
            }
            if (base.contains("code") || base.contains("vscodium")) && command_exists("code") {
                return Some(Editor::new("code", Kind::VsCode));
            }
            if base.contains("nvim") || base.contains("neovim") {
                return command_exists("nvim").then(|| Editor::new("nvim", Kind::Neovim));
            }
            if base.contains("vim") || base.contains("gvim") {
                return command_exists("vim").then(|| Editor::new("vim", Kind::Vim));
            }
            if base == "hx" || base.contains("helix") {
                return command_exists("hx").then(|| Editor::new("hx", Kind::Helix));
            }
            None
        }

        let mut pid = std::process::id() as i32;
        for _ in 0..12 {
            let Some(parent) = parent_of(pid) else {
                break;
            };
            if parent <= 1 {
                break;
            }
            if let Some(comm) = comm_of(parent)
                && let Some(editor) = editor_from_comm(&comm)
            {
                return Some(editor);
            }
            pid = parent;
        }
    }
    None
}

/// The current `$VISUAL`/`$EDITOR`, if it names something usable.
fn env_fallback() -> Option<Editor> {
    for var in ["VISUAL", "EDITOR"] {
        if let Ok(program) = std::env::var(var)
            && let Some(editor) = from_env_value(program)
        {
            return Some(editor);
        }
    }
    None
}

fn kind_for(bin: &str) -> Kind {
    let bin = bin.to_lowercase();
    if bin.contains("zed") {
        Kind::Zed
    } else if bin.contains("code")
        || bin.contains("cursor")
        || bin.contains("windsurf")
        || bin.contains("codium")
    {
        Kind::VsCode
    } else if bin.contains("nvim") || bin.contains("neovim") {
        Kind::Neovim
    } else if bin.contains("vim") {
        Kind::Vim
    } else if bin.contains("helix") || bin.contains("hx") {
        Kind::Helix
    } else {
        Kind::Custom
    }
}

fn command_exists(bin: &str) -> bool {
    std::process::Command::new("which")
        .arg(bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
