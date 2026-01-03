use anyhow::Result;
use console::{style, Emoji};
use std::io::{self, Write};
use std::process::Command;

// Beautiful emoji icons
static CHECKMARK: Emoji<'_, '_> = Emoji("✔ ", "√ ");
static CROSS: Emoji<'_, '_> = Emoji("✖ ", "x ");
static ARROW: Emoji<'_, '_> = Emoji("→ ", "-> ");
static PACKAGE: Emoji<'_, '_> = Emoji("📦 ", "");
static COMPUTER: Emoji<'_, '_> = Emoji("💻 ", "");
static WRENCH: Emoji<'_, '_> = Emoji("🔧 ", "");
static SPARKLES: Emoji<'_, '_> = Emoji("✨ ", "* ");
static WARNING: Emoji<'_, '_> = Emoji("⚠️  ", "! ");
static INFO: Emoji<'_, '_> = Emoji("ℹ️  ", "i ");
static ROCKET: Emoji<'_, '_> = Emoji("🚀 ", "");
static GEAR: Emoji<'_, '_> = Emoji("⚙️  ", "");

/// Arguments for the doctor command
#[derive(Debug, Clone)]
pub struct DoctorArgs {
    pub fix: bool,
}

/// Check status of a tool
struct ToolStatus {
    name: &'static str,
    found: bool,
    version: Option<String>,
    path: Option<String>,
}

/// Detected operating system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Os {
    MacOS,
    Linux,
    Windows,
    FreeBSD,
    Unknown,
}

impl Os {
    fn detect() -> Self {
        if cfg!(target_os = "macos") {
            Os::MacOS
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else if cfg!(target_os = "freebsd") {
            Os::FreeBSD
        } else {
            Os::Unknown
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Os::MacOS => "macOS",
            Os::Linux => "Linux",
            Os::Windows => "Windows",
            Os::FreeBSD => "FreeBSD",
            Os::Unknown => "Unknown OS",
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            Os::MacOS => "🍎",
            Os::Linux => "🐧",
            Os::Windows => "🪟",
            Os::FreeBSD => "😈",
            Os::Unknown => "❓",
        }
    }
}

/// Detect macOS architecture for correct Homebrew path
fn get_homebrew_prefix() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "/opt/homebrew"
    } else {
        "/usr/local"
    }
}

/// Detect Linux distribution
fn detect_linux_distro() -> Option<String> {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("ID=") {
                let id = line.trim_start_matches("ID=").trim_matches('"');
                return Some(id.to_lowercase());
            }
        }
    }

    if std::path::Path::new("/etc/debian_version").exists() {
        return Some("debian".to_string());
    }
    if std::path::Path::new("/etc/redhat-release").exists() {
        return Some("rhel".to_string());
    }
    if std::path::Path::new("/etc/arch-release").exists() {
        return Some("arch".to_string());
    }

    None
}

/// Check if running as root/admin
fn is_root() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(windows)]
    {
        Command::new("net")
            .args(["session"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// Check if a command exists
fn command_exists(cmd: &str) -> bool {
    if cfg!(target_os = "windows") {
        Command::new("where")
            .arg(cmd)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        Command::new("which")
            .arg(cmd)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// Check if a command exists and get its version
fn check_tool(name: &'static str, version_args: &[&str]) -> ToolStatus {
    let path = if cfg!(target_os = "windows") {
        Command::new("where").arg(name).output().ok()
    } else {
        Command::new("which").arg(name).output().ok()
    }
    .and_then(|output| {
        if output.status.success() {
            String::from_utf8(output.stdout)
                .ok()
                .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
                .filter(|s| !s.is_empty())
        } else {
            None
        }
    });

    let found = path.is_some();

    let version = if found {
        Command::new(name)
            .args(version_args)
            .output()
            .ok()
            .and_then(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let version_str = if stdout.trim().is_empty() {
                    &stderr
                } else {
                    &stdout
                };
                version_str
                    .lines()
                    .next()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
    } else {
        None
    };

    ToolStatus {
        name,
        found,
        version,
        path,
    }
}

/// Print a beautiful header
fn print_header() {
    let width = 35;
    let title = "Supamigrate Doctor";
    let padding = width - title.len() - 2; // -2 for spaces around title

    println!();
    println!("  ╭{}╮", "─".repeat(width));
    println!(
        "  │ {}{} │",
        style(title).bold().white(),
        " ".repeat(padding)
    );
    println!("  ╰{}╯", "─".repeat(width));
    println!();
}

/// Print a section header
fn print_section(title: &str, emoji: Emoji<'_, '_>) {
    println!("  {} {}", emoji, style(title).bold().underlined());
    println!();
}

/// Print system information
fn print_system_info(os: Os, distro: Option<&str>, pkg_manager: Option<&str>) {
    print_section("System", COMPUTER);

    println!("     {}  {}", os.emoji(), style(os.name()).white().bold());

    if let Some(d) = distro {
        println!(
            "        {} {}",
            style("Distribution:").dim(),
            style(d).white()
        );
    }

    // Show architecture for macOS
    if os == Os::MacOS {
        let arch = if cfg!(target_arch = "aarch64") {
            "Apple Silicon"
        } else {
            "Intel"
        };
        println!(
            "        {} {}",
            style("Architecture:").dim(),
            style(arch).white()
        );
    }

    if let Some(pm) = pkg_manager {
        println!(
            "        {} {}",
            style("Package Manager:").dim(),
            style(pm).white()
        );
    }

    if is_root() {
        println!(
            "        {} {}",
            style("Privileges:").dim(),
            style("root").yellow()
        );
    }

    println!();
}

/// Print tool status with beautiful formatting
fn print_tool_status(tool: &ToolStatus, required: bool) {
    if tool.found {
        let version = tool.version.as_deref().unwrap_or("installed");
        // Extract just the version number if possible
        let short_version = extract_version(version);

        println!(
            "     {}{}  {}",
            CHECKMARK,
            style(tool.name).green().bold(),
            style(format!("v{}", short_version)).dim()
        );

        if let Some(path) = &tool.path {
            println!("        {} {}", ARROW, style(path).dim());
        }
    } else {
        let status = if required { "missing" } else { "not found" };
        println!(
            "     {}{}  {}",
            CROSS,
            style(tool.name).red().bold(),
            style(status).red().dim()
        );
    }
}

/// Extract version number from version string
fn extract_version(version: &str) -> String {
    // Try to extract version like "14.20" from "pg_dump (PostgreSQL) 14.20 (Homebrew)"
    // Or "475" from "Apple gzip 475"
    let parts: Vec<&str> = version.split_whitespace().collect();

    for part in parts.iter().rev() {
        // Look for a version-like pattern (starts with digit)
        let clean = part.trim_matches(|c| c == '(' || c == ')');
        if clean
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            return clean.to_string();
        }
    }

    // Fallback: return first 20 chars
    version.chars().take(20).collect()
}

/// Print tools section
fn print_tools(required: &[ToolStatus], optional: &[ToolStatus]) {
    print_section("Dependencies", PACKAGE);

    println!("     {}", style("Required").white().bold());
    for tool in required {
        print_tool_status(tool, true);
    }

    println!();
    println!("     {}", style("Optional").white().bold());
    for tool in optional {
        print_tool_status(tool, false);
    }

    println!();
}

/// Print success message
fn print_success() {
    let width = 35;
    let text = "All systems go!";
    let content_len = 3 + text.len(); // emoji width ~2 + space + text
    let padding = width - content_len - 2;

    println!("  {}", style(format!("╭{}╮", "─".repeat(width))).green());
    println!(
        "  {} {}{}{} {}",
        style("│").green(),
        SPARKLES,
        style(text).green().bold(),
        " ".repeat(padding),
        style("│").green()
    );
    println!("  {}", style(format!("╰{}╯", "─".repeat(width))).green());
    println!();
    println!("     {}Ready to migrate your Supabase projects.", ROCKET);
    println!();
}

/// Print failure message
fn print_failure(missing: &[&str]) {
    let width = 35;
    let text = "Missing dependencies";
    let prefix_len = 3; // emoji width ~2 + space
    let content_len = prefix_len + text.len();
    let padding = width - content_len - 2;

    println!("  {}", style(format!("╭{}╮", "─".repeat(width))).red());
    println!(
        "  {} {}{}{} {}",
        style("│").red(),
        WARNING,
        style(text).red().bold(),
        " ".repeat(padding),
        style("│").red()
    );
    println!("  {}", style(format!("╰{}╯", "─".repeat(width))).red());
    println!();
    println!("     {}The following tools are required:", INFO);
    for tool in missing {
        println!(
            "        {} {}",
            style("•").red(),
            style(*tool).white().bold()
        );
    }
    println!();
}

/// Print installation instructions
fn print_install_instructions(os: Os, distro: Option<&str>) {
    print_section("Installation", WRENCH);

    let instructions = get_install_instructions(os, distro);
    for line in instructions.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.trim().starts_with("Install")
            || line.trim().starts_with("Option")
            || line.trim().starts_with("For ")
            || line.trim().starts_with("Add to")
            || line.trim().starts_with("Or ")
        {
            println!("     {}", style(line.trim()).white());
        } else if line.trim().starts_with("Note:")
            || line.trim().starts_with("After")
            || line.trim().starts_with("During")
            || line.trim().starts_with("Then")
        {
            println!("     {}", style(line.trim()).dim());
        } else if line.contains("://") {
            println!("       {}", style(line.trim()).cyan().underlined());
        } else {
            println!("       {}", style(line.trim()).yellow());
        }
    }
    println!();
}

/// Print tip
fn print_tip(message: &str) {
    println!(
        "     {} {}",
        style("Tip:").cyan().bold(),
        style(message).dim()
    );
    println!();
}

/// Get installation instructions for PostgreSQL client tools
fn get_install_instructions(os: Os, distro: Option<&str>) -> String {
    match os {
        Os::MacOS => {
            let prefix = get_homebrew_prefix();
            format!(
                r#"Install via Homebrew (recommended):
  brew install libpq
  brew link --force libpq

Or install full PostgreSQL:
  brew install postgresql@16

Note: If pg_dump is still not found, add to PATH:
  echo 'export PATH="{prefix}/opt/libpq/bin:$PATH"' >> ~/.zshrc"#
            )
        }
        Os::Linux => match distro {
            Some("ubuntu") | Some("debian") | Some("pop") | Some("mint") | Some("elementary")
            | Some("linuxmint") => r#"Install via apt:
  sudo apt update && sudo apt install postgresql-client"#
                .to_string(),
            Some("fedora") => r#"Install via dnf:
  sudo dnf install postgresql"#
                .to_string(),
            Some("rhel") | Some("centos") | Some("rocky") | Some("alma") | Some("ol") => {
                r#"Install via yum/dnf:
  sudo dnf install postgresql"#
                    .to_string()
            }
            Some("arch") | Some("manjaro") | Some("endeavouros") | Some("garuda") => {
                r#"Install via pacman:
  sudo pacman -S postgresql-libs"#
                    .to_string()
            }
            Some("opensuse")
            | Some("opensuse-leap")
            | Some("opensuse-tumbleweed")
            | Some("suse")
            | Some("sles") => r#"Install via zypper:
  sudo zypper install postgresql"#
                .to_string(),
            Some("alpine") => r#"Install via apk:
  apk add postgresql-client"#
                .to_string(),
            Some("nixos") => r#"Add to configuration.nix:
  environment.systemPackages = [ pkgs.postgresql ];

Then rebuild:
  sudo nixos-rebuild switch"#
                .to_string(),
            Some("gentoo") => r#"Install via emerge:
  sudo emerge --ask dev-db/postgresql"#
                .to_string(),
            Some("void") => r#"Install via xbps:
  sudo xbps-install postgresql-client"#
                .to_string(),
            _ => r#"For Debian/Ubuntu:
  sudo apt install postgresql-client

For Fedora/RHEL:
  sudo dnf install postgresql

For Arch Linux:
  sudo pacman -S postgresql-libs"#
                .to_string(),
        },
        Os::Windows => r#"Option 1 - Chocolatey (recommended):
  choco install postgresql

Option 2 - Scoop:
  scoop install postgresql

Option 3 - winget:
  winget install PostgreSQL.PostgreSQL

After installation, add to PATH:
  C:\Program Files\PostgreSQL\16\bin"#
            .to_string(),
        Os::FreeBSD => r#"Install via pkg:
  sudo pkg install postgresql16-client"#
            .to_string(),
        Os::Unknown => r#"Please install PostgreSQL client tools for your OS.
  https://www.postgresql.org/download/"#
            .to_string(),
    }
}

/// Get installation command for PostgreSQL client tools
fn get_install_command(os: Os, distro: Option<&str>) -> Option<(String, Vec<String>)> {
    match os {
        Os::MacOS => {
            if command_exists("brew") {
                Some((
                    "brew".to_string(),
                    vec!["install".to_string(), "libpq".to_string()],
                ))
            } else {
                None
            }
        }
        Os::Linux => {
            let use_sudo = !is_root();
            let mut base_cmd: Vec<String> = if use_sudo {
                vec!["sudo".to_string()]
            } else {
                vec![]
            };

            match distro {
                Some("ubuntu") | Some("debian") | Some("pop") | Some("mint")
                | Some("elementary") | Some("linuxmint") => {
                    if command_exists("apt-get") {
                        base_cmd.extend([
                            "apt-get".to_string(),
                            "install".to_string(),
                            "-y".to_string(),
                            "-qq".to_string(),
                            "postgresql-client".to_string(),
                        ]);
                        Some((base_cmd.remove(0), base_cmd))
                    } else {
                        None
                    }
                }
                Some("fedora") => {
                    if command_exists("dnf") {
                        base_cmd.extend([
                            "dnf".to_string(),
                            "install".to_string(),
                            "-y".to_string(),
                            "postgresql".to_string(),
                        ]);
                        Some((base_cmd.remove(0), base_cmd))
                    } else {
                        None
                    }
                }
                Some("rhel") | Some("centos") | Some("rocky") | Some("alma") | Some("ol") => {
                    if command_exists("dnf") {
                        base_cmd.extend([
                            "dnf".to_string(),
                            "install".to_string(),
                            "-y".to_string(),
                            "postgresql".to_string(),
                        ]);
                        Some((base_cmd.remove(0), base_cmd))
                    } else if command_exists("yum") {
                        base_cmd.extend([
                            "yum".to_string(),
                            "install".to_string(),
                            "-y".to_string(),
                            "postgresql".to_string(),
                        ]);
                        Some((base_cmd.remove(0), base_cmd))
                    } else {
                        None
                    }
                }
                Some("arch") | Some("manjaro") | Some("endeavouros") | Some("garuda") => {
                    if command_exists("pacman") {
                        base_cmd.extend([
                            "pacman".to_string(),
                            "-S".to_string(),
                            "--noconfirm".to_string(),
                            "postgresql-libs".to_string(),
                        ]);
                        Some((base_cmd.remove(0), base_cmd))
                    } else {
                        None
                    }
                }
                Some("opensuse")
                | Some("opensuse-leap")
                | Some("opensuse-tumbleweed")
                | Some("suse")
                | Some("sles") => {
                    if command_exists("zypper") {
                        base_cmd.extend([
                            "zypper".to_string(),
                            "--non-interactive".to_string(),
                            "install".to_string(),
                            "postgresql".to_string(),
                        ]);
                        Some((base_cmd.remove(0), base_cmd))
                    } else {
                        None
                    }
                }
                Some("alpine") => {
                    if command_exists("apk") {
                        if is_root() {
                            Some((
                                "apk".to_string(),
                                vec![
                                    "add".to_string(),
                                    "--no-cache".to_string(),
                                    "postgresql-client".to_string(),
                                ],
                            ))
                        } else {
                            Some((
                                "sudo".to_string(),
                                vec![
                                    "apk".to_string(),
                                    "add".to_string(),
                                    "--no-cache".to_string(),
                                    "postgresql-client".to_string(),
                                ],
                            ))
                        }
                    } else {
                        None
                    }
                }
                Some("void") => {
                    if command_exists("xbps-install") {
                        base_cmd.extend([
                            "xbps-install".to_string(),
                            "-y".to_string(),
                            "postgresql-client".to_string(),
                        ]);
                        Some((base_cmd.remove(0), base_cmd))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        Os::Windows => {
            if command_exists("choco") {
                Some((
                    "choco".to_string(),
                    vec![
                        "install".to_string(),
                        "postgresql".to_string(),
                        "-y".to_string(),
                    ],
                ))
            } else if command_exists("scoop") {
                Some((
                    "scoop".to_string(),
                    vec!["install".to_string(), "postgresql".to_string()],
                ))
            } else if command_exists("winget") {
                Some((
                    "winget".to_string(),
                    vec![
                        "install".to_string(),
                        "PostgreSQL.PostgreSQL".to_string(),
                        "--silent".to_string(),
                        "--accept-package-agreements".to_string(),
                        "--accept-source-agreements".to_string(),
                    ],
                ))
            } else {
                None
            }
        }
        Os::FreeBSD => {
            if command_exists("pkg") {
                if is_root() {
                    Some((
                        "pkg".to_string(),
                        vec![
                            "install".to_string(),
                            "-y".to_string(),
                            "postgresql16-client".to_string(),
                        ],
                    ))
                } else {
                    Some((
                        "sudo".to_string(),
                        vec![
                            "pkg".to_string(),
                            "install".to_string(),
                            "-y".to_string(),
                            "postgresql16-client".to_string(),
                        ],
                    ))
                }
            } else {
                None
            }
        }
        Os::Unknown => None,
    }
}

/// Check if a package manager is available
fn check_package_manager(os: Os, distro: Option<&str>) -> Option<&'static str> {
    match os {
        Os::MacOS => command_exists("brew").then_some("Homebrew"),
        Os::Linux => match distro {
            Some("ubuntu") | Some("debian") | Some("pop") | Some("mint") | Some("elementary")
            | Some("linuxmint") => command_exists("apt").then_some("apt"),
            Some("fedora") => command_exists("dnf").then_some("dnf"),
            Some("rhel") | Some("centos") | Some("rocky") | Some("alma") | Some("ol") => {
                if command_exists("dnf") {
                    Some("dnf")
                } else if command_exists("yum") {
                    Some("yum")
                } else {
                    None
                }
            }
            Some("arch") | Some("manjaro") | Some("endeavouros") | Some("garuda") => {
                command_exists("pacman").then_some("pacman")
            }
            Some("opensuse")
            | Some("opensuse-leap")
            | Some("opensuse-tumbleweed")
            | Some("suse")
            | Some("sles") => command_exists("zypper").then_some("zypper"),
            Some("alpine") => command_exists("apk").then_some("apk"),
            Some("nixos") => Some("nix"),
            Some("gentoo") => command_exists("emerge").then_some("portage"),
            Some("void") => command_exists("xbps-install").then_some("xbps"),
            _ => None,
        },
        Os::Windows => {
            if command_exists("choco") {
                Some("Chocolatey")
            } else if command_exists("scoop") {
                Some("Scoop")
            } else if command_exists("winget") {
                Some("winget")
            } else {
                None
            }
        }
        Os::FreeBSD => command_exists("pkg").then_some("pkg"),
        Os::Unknown => None,
    }
}

/// Attempt to install PostgreSQL client tools
fn install_pg_tools(os: Os, distro: Option<&str>) -> Result<bool> {
    let Some((cmd, args)) = get_install_command(os, distro) else {
        println!(
            "     {}Cannot auto-install: no supported package manager found.",
            WARNING
        );
        return Ok(false);
    };

    println!();
    print_section("Installing", GEAR);
    println!(
        "     {} {}",
        ARROW,
        style(format!("{} {}", cmd, args.join(" "))).yellow()
    );
    println!();

    let status = Command::new(&cmd).args(&args).status()?;

    if status.success() {
        if os == Os::MacOS {
            println!();
            println!("     {}Creating symlinks...", GEAR);
            let _ = Command::new("brew")
                .args(["link", "--force", "libpq"])
                .status();
        }

        if os == Os::Windows {
            println!();
            println!("     {}You may need to restart your terminal.", INFO);
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

/// Prompt user for confirmation
fn confirm(prompt: &str) -> bool {
    print!(
        "     {} {} ",
        style("?").cyan().bold(),
        style(prompt).white()
    );
    print!("{}", style("[y/N] ").dim());
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

pub fn run(args: DoctorArgs) -> Result<()> {
    let os = Os::detect();
    let distro = if os == Os::Linux {
        detect_linux_distro()
    } else {
        None
    };
    let distro_ref = distro.as_deref();
    let pkg_manager = check_package_manager(os, distro_ref);

    // Header
    print_header();

    // System info
    print_system_info(os, distro_ref, pkg_manager);

    // Check tools
    let pg_dump = check_tool("pg_dump", &["--version"]);
    let psql = check_tool("psql", &["--version"]);
    let gzip = check_tool("gzip", &["--version"]);
    let gunzip = check_tool("gunzip", &["--version"]);

    let required = vec![pg_dump, psql];
    let optional = vec![gzip, gunzip];

    print_tools(&required, &optional);

    // Check if all required tools are found
    let missing: Vec<&str> = required
        .iter()
        .filter(|t| !t.found)
        .map(|t| t.name)
        .collect();

    if missing.is_empty() {
        print_success();
        return Ok(());
    }

    // Missing tools
    print_failure(&missing);

    // Try to auto-install
    if args.fix {
        if get_install_command(os, distro_ref).is_some() {
            if install_pg_tools(os, distro_ref)? {
                println!();
                println!(
                    "     {}{}",
                    SPARKLES,
                    style("Installation complete!").green().bold()
                );
                println!();
                print_tip("Run 'supamigrate doctor' again to verify.");
                return Ok(());
            } else {
                println!();
                println!("     {}{}", CROSS, style("Installation failed.").red());
            }
        } else {
            println!("     {}No supported package manager detected.", WARNING);
        }
        println!();
    } else if get_install_command(os, distro_ref).is_some() {
        if confirm("Install missing dependencies now?") {
            println!();
            if install_pg_tools(os, distro_ref)? {
                println!();
                println!(
                    "     {}{}",
                    SPARKLES,
                    style("Installation complete!").green().bold()
                );
                println!();
                print_tip("Run 'supamigrate doctor' again to verify.");
                return Ok(());
            } else {
                println!();
                println!("     {}{}", CROSS, style("Installation failed.").red());
            }
        }
        println!();
    }

    // Show manual instructions
    print_install_instructions(os, distro_ref);

    if !args.fix && get_install_command(os, distro_ref).is_some() {
        print_tip("Run 'supamigrate doctor --fix' for automatic installation.");
    }

    std::process::exit(1);
}
