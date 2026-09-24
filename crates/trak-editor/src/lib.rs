//! Open a `file:line` location in the user's editor.

use std::path::Path;

/// A resolved editor: the binary to run plus how it should be launched.
#[derive(Debug, Clone)]
pub struct Editor {
    name: String,
    program: String,
    /// GUI editors (code, zed) are launched detached; terminal editors run
    /// in the foreground.
    gui: bool,
}

impl Editor {
    /// Pick an editor: `TRAK_EDITOR`, then `$EDITOR`, then a known binary.
    pub fn detect() -> Option<Editor> {
        for var in ["TRAK_EDITOR", "EDITOR"] {
            if let Ok(program) = std::env::var(var)
                && !program.trim().is_empty()
            {
                return Some(Editor {
                    name: program.clone(),
                    program,
                    gui: false,
                });
            }
        }

        let candidates: &[(&str, &str, bool)] = &[
            ("Visual Studio Code", "code", true),
            ("Zed", "zed", true),
            ("Neovim", "nvim", false),
            ("Vim", "vim", false),
        ];
        for (name, bin, gui) in candidates {
            if command_exists(bin) {
                return Some(Editor {
                    name: (*name).to_string(),
                    program: (*bin).to_string(),
                    gui: *gui,
                });
            }
        }
        None
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Open `file` at `line` in this editor.
    pub fn open(&self, file: &str, line: usize) -> std::io::Result<()> {
        let program_name = Path::new(&self.program)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&self.program);

        let (program, args) = match program_name {
            "code" => (
                self.program.clone(),
                vec![String::from("-g"), format!("{file}:{line}")],
            ),
            "zed" => (self.program.clone(), vec![format!("{file}:{line}")]),
            "nvim" | "vim" => (
                self.program.clone(),
                vec![format!("+{line}"), file.to_string()],
            ),
            _ => {
                let mut parts = self.program.split_whitespace();
                let program = parts.next().unwrap_or("vi").to_string();
                let mut args: Vec<String> = parts.map(String::from).collect();
                // Terminal editors most commonly understand `+line file`.
                if self.gui {
                    args.push(format!("{file}:{line}"));
                } else {
                    args.push(format!("+{line}"));
                    args.push(file.to_string());
                }
                (program, args)
            }
        };

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
