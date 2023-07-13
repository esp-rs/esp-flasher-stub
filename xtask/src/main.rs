use std::{
    fs,
    iter,
    path::{Path, PathBuf},
    process::{exit, Command, ExitStatus, Stdio},
};

use anyhow::{anyhow, bail, Result};
use cargo_metadata::Message;
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;
use strum::Display;
use xmas_elf::ElfFile;

#[derive(Debug, Clone, Copy, PartialEq, Display, ValueEnum)]
#[strum(serialize_all = "lowercase")]
enum Chip {
    Esp32,
    Esp32c2,
    Esp32c3,
    Esp32s2,
    Esp32s3,
}

impl Chip {
    pub fn toolchain(&self) -> &'static str {
        match self {
            Chip::Esp32c2 | Chip::Esp32c3 => "+nightly",
            Chip::Esp32 | Chip::Esp32s2 | Chip::Esp32s3 => "+esp",
        }
    }

    pub fn target(&self) -> &'static str {
        match self {
            Chip::Esp32 => "xtensa-esp32-none-elf",
            Chip::Esp32c2 | Chip::Esp32c3 => "riscv32imc-unknown-none-elf",
            Chip::Esp32s2 => "xtensa-esp32s2-none-elf",
            Chip::Esp32s3 => "xtensa-esp32s3-none-elf",
        }
    }
}

#[derive(Debug, Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Build the flasher stub for the specified chip(s)
    Build {
        #[clap(value_enum)]
        chips: Vec<Chip>,
    },

    /// Build the flasher stub for the specified chip(s) and convert it to JSON
    Wrap {
        #[clap(value_enum)]
        chips: Vec<Chip>,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::new()
        .filter_module("xtask", log::LevelFilter::Info)
        .init();

    // The directory containing the cargo manifest for the 'xtask' package is a
    // subdirectory within the cargo workspace.
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = workspace.parent().unwrap().canonicalize()?;

    match Cli::parse().command {
        Commands::Build { chips } => chips
            .iter()
            .try_for_each(|chip| build(&workspace, chip).map(|_| ())),
        Commands::Wrap { chips } => chips.iter().try_for_each(|chip| wrap(&workspace, chip)),
    }
}

fn build(workspace: &Path, chip: &Chip) -> Result<PathBuf> {
    // Invoke the 'cargo build' command, passing our list of arguments.
    let output = Command::new("cargo")
        .args([
            &format!("{}", chip.toolchain()),
            "build",
            "-Zbuild-std=core",
            "--release",
            &format!("--target={}", chip.target()),
            &format!("--features={chip},dprint"),
        ])
        .args(["--message-format", "json-diagnostic-rendered-ansi"])
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?
        .wait_with_output()?;

    // Parse build output.
    let messages = Message::parse_stream(&output.stdout[..]);

    // Find target artifact.
    let mut target_artifact = None;

    for message in messages {
        let message = message?;

        match message {
            Message::CompilerArtifact(artifact) => {
                if artifact.executable.is_some() {
                    if target_artifact.is_some() {
                        bail!("Multiple build artifacts found!");
                    } else {
                        target_artifact = Some(artifact);
                    }
                }
            }
            Message::CompilerMessage(message) => {
                if let Some(rendered) = message.message.rendered {
                    print!("{}", rendered);
                }
            }
            // Ignore all other messages.
            _ => (),
        }
    }

    // Check if the command succeeded, otherwise return an error. Any error messages
    // occurring during the build are shown above, when the compiler messages are
    // rendered.
    if !output.status.success() {
        exit_with_process_status(output.status);
    }

    // If no target artifact was found, we don't have a path to return.
    let target_artifact = target_artifact.ok_or(anyhow!("No build artifact found!"))?;
    let artifact_path: PathBuf = target_artifact.executable.unwrap().into();

    log::info!("{}", artifact_path.display());
    Ok(artifact_path)
}

fn wrap(workspace: &Path, chip: &Chip) -> Result<()> {
    use base64::engine::{general_purpose, Engine};

    // ordering here matters! should be in order of placement in RAM
    let text_sections = [".text_init", ".text", ".trap", ".rwtext"];
    let data_sections = [".rodata", ".data"];

    let artifact_path = build(workspace, chip)?;

    let elf_data = fs::read(artifact_path)?;
    let elf = ElfFile::new(&elf_data).unwrap();

    let entry = elf.header.pt2.entry_point();

    let mut text = Vec::new();
    let mut text_start = 0;

    for (i, t) in text_sections.iter().enumerate() {
        let text_section = elf.find_section_by_name(t).unwrap();
        let next_t = text_sections.get(i + 1);
        if text_start == 0 {
            text_start = text_section.address();
            log::info!("Found start: 0x{:08X}", text_start)
        }
        let mut text_data = text_section.raw_data(&elf).to_vec();
        if let Some(next) = next_t {
            let next_text_section = elf.find_section_by_name(next).unwrap();
            let end = text_section.address() as usize + text_data.len();
            let padding = next_text_section.address() as usize - end;
            log::info!("Size of padding to next section: {}", padding);
            text_data.extend(iter::repeat('\0' as u8).take(padding));
        }
        text.extend(&text_data);
    }
    let text = general_purpose::STANDARD.encode(&text);

    let mut data = Vec::new();
    let mut data_start = 0;

    for (i, t) in data_sections.iter().enumerate() {
        let data_section = elf.find_section_by_name(t).unwrap();
        let next_t = data_sections.get(i + 1);
        if data_start == 0 {
            data_start = data_section.address();
            log::info!("Found start: 0x{:08X}", data_start)
        }
        let mut data_data = data_section.raw_data(&elf).to_vec();
        if let Some(next) = next_t {
            let next_data_section = elf.find_section_by_name(next).unwrap();
            if next_data_section.raw_data(&elf).len() != 0 {
                let end = data_section.address() as usize + data_data.len();
                let padding = next_data_section.address() as usize - end;
                log::info!("Size of padding to next section: {}", padding);
                data_data.extend(iter::repeat('\0' as u8).take(padding));
            }
        }
        data.extend(&data_data);
    }
    let data = general_purpose::STANDARD.encode(&data);

    let stub = json!({
        "entry": entry,
        "text": text,
        "text_start": text_start,
        "data": data,
        "data_start": data_start,
    });

    let stub_file = workspace.join(format!("{chip}.json"));
    let contents = serde_json::to_string(&stub)?;
    fs::write(stub_file, &contents)?;

    let stub_file = workspace.join(format!("{chip}.toml"));
    let stub: toml::Value = serde_json::from_str(&contents)?;
    let contents = toml::to_string(&stub)?;
    fs::write(stub_file, contents)?;

    Ok(())
}

fn exit_with_process_status(status: ExitStatus) -> ! {
    #[cfg(unix)]
    let code = {
        use std::os::unix::process::ExitStatusExt;
        let code = status.code().or_else(|| status.signal()).unwrap_or(1);

        code
    };

    #[cfg(not(unix))]
    let code = status.code().unwrap_or(1);

    exit(code)
}
