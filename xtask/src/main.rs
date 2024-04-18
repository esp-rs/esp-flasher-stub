use std::{
    fs,
    iter,
    path::{Path, PathBuf},
    process::{exit, Command, ExitStatus, Stdio},
};

use anyhow::{anyhow, bail, Result};
use cargo_metadata::Message;
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, IntoEnumIterator};
use xmas_elf::{sections::SectionHeader, ElfFile};

#[derive(Debug, Clone, Copy, PartialEq, Display, EnumIter, ValueEnum)]
#[strum(serialize_all = "lowercase")]
enum Chip {
    Esp32,
    Esp32c2,
    Esp32c3,
    Esp32c6,
    Esp32h2,
    Esp32s2,
    Esp32s3,
}

impl Chip {
    pub fn toolchain(&self) -> &'static str {
        match self {
            Chip::Esp32 | Chip::Esp32s2 | Chip::Esp32s3 => "+esp",
            _ => "+nightly",
        }
    }

    pub fn target(&self) -> &'static str {
        match self {
            Chip::Esp32 => "xtensa-esp32-none-elf",
            Chip::Esp32c2 | Chip::Esp32c3 => "riscv32imc-unknown-none-elf",
            Chip::Esp32c6 | Chip::Esp32h2 => "riscv32imac-unknown-none-elf",
            Chip::Esp32s2 => "xtensa-esp32s2-none-elf",
            Chip::Esp32s3 => "xtensa-esp32s3-none-elf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Display, ValueEnum)]
#[strum(serialize_all = "lowercase")]
enum Format {
    Json,
    Toml,
}

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Build the flasher stub for the specified chip(s)
    Build {
        #[arg(value_enum, default_values_t = Chip::iter())]
        chips: Vec<Chip>,

        #[arg(long)]
        dprint: bool,
    },

    /// Build the flasher stub for the specified chip(s) and convert it to JSON
    Wrap {
        #[arg(long, value_enum)]
        format: Option<Format>,

        #[arg(value_enum, default_values_t = Chip::iter())]
        chips: Vec<Chip>,

        #[arg(long)]
        dprint: bool,
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
        Commands::Build { chips, dprint } => chips
            .iter()
            .try_for_each(|chip| build(&workspace, chip, dprint).map(|_| ())),
        Commands::Wrap {
            chips,
            format,
            dprint,
        } => chips
            .iter()
            .try_for_each(|chip| wrap(&workspace, chip, dprint, format)),
    }
}

fn build(workspace: &Path, chip: &Chip, dprint: bool) -> Result<PathBuf> {
    // Invoke the 'cargo build' command, passing our list of arguments.
    let features = if dprint {
        format!("--features={chip},dprint")
    } else {
        format!("--features={chip}")
    };
    let output = Command::new("cargo")
        .args([
            chip.toolchain(),
            "build",
            "-Zbuild-std=core",
            "-Zbuild-std-features=panic_immediate_abort",
            "--release",
            &format!("--target={}", chip.target()),
            &features,
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

#[derive(Debug, Deserialize, Serialize)]
struct Stub {
    entry: u64,
    text: String,
    text_start: u64,
    data: String,
    data_start: u64,
}

fn wrap(workspace: &Path, chip: &Chip, dprint: bool, format: Option<Format>) -> Result<()> {
    use base64::engine::{general_purpose, Engine};

    // ordering here matters! should be in order of placement in RAM
    // note that sections that don't exists, or contain no data are ignored
    let text_sections = [
        ".vectors",
        ".text_init",
        ".text",
        ".trap",
        ".init",
        ".fini",
        ".rwtext",
    ];
    let data_sections = [".rodata", ".data"];

    let artifact_path = build(workspace, chip, dprint)?;

    let elf_data = fs::read(artifact_path)?;
    let elf = ElfFile::new(&elf_data).unwrap();

    let entry = elf.header.pt2.entry_point();

    let (text_start, text) = concat_sections(&elf, &text_sections);
    let text = general_purpose::STANDARD.encode(text);

    let (data_start, data) = concat_sections(&elf, &data_sections);
    let data = general_purpose::STANDARD.encode(data);

    log::info!("Total size of stub is {}bytes", text.len() + data.len());

    let stub = Stub {
        entry,
        text,
        text_start,
        data,
        data_start,
    };

    match format {
        Some(Format::Json) => write_json(workspace, chip, &stub)?,
        Some(Format::Toml) => write_toml(workspace, chip, &stub)?,
        None => {
            write_json(workspace, chip, &stub)?;
            write_toml(workspace, chip, &stub)?;
        }
    }

    Ok(())
}

fn write_json(workspace: &Path, chip: &Chip, stub: &Stub) -> Result<()> {
    let stub_file = workspace.join(format!("{chip}.json"));
    let contents = serde_json::to_string(&stub)?;

    log::info!("Writing JSON stub: {}", stub_file.display());
    fs::write(stub_file, contents)?;

    Ok(())
}

fn write_toml(workspace: &Path, chip: &Chip, stub: &Stub) -> Result<()> {
    let stub_file = workspace.join(format!("{chip}.toml"));
    let contents = toml::to_string(stub)?;

    log::info!("Writing TOML stub: {}", stub_file.display());
    fs::write(stub_file, contents)?;

    Ok(())
}

fn exit_with_process_status(status: ExitStatus) -> ! {
    #[cfg(unix)]
    let code = {
        use std::os::unix::process::ExitStatusExt;
        status.code().or_else(|| status.signal()).unwrap_or(1)
    };

    #[cfg(not(unix))]
    let code = status.code().unwrap_or(1);

    exit(code)
}

fn concat_sections(elf: &ElfFile, list: &[&str]) -> (u64, Vec<u8>) {
    let mut data = Vec::new();
    let mut data_start = 0;

    let sections: Vec<SectionHeader> = list
        .iter()
        .filter_map(|name| elf.find_section_by_name(name))
        .filter(|s| !s.raw_data(elf).is_empty())
        .collect();

    for (i, section) in sections.iter().enumerate() {
        let next_t = sections.get(i + 1);
        if data_start == 0 {
            data_start = section.address();
            log::debug!("Found start: 0x{:08X}", data_start)
        }
        let mut data_data = section.raw_data(elf).to_vec();
        let padding = if let Some(next) = next_t {
            assert!(
                section.address() < next.address(),
                "Sections should be listed in ascending order. Current: 0x{:08X}, next: 0x{:08X}.",
                section.address(),
                next.address()
            );
            let end = section.address() as usize + data_data.len();
            let padding = next.address() as usize - end;
            log::debug!("Size of padding to next section: {}", padding);
            if padding > 512 {
                log::warn!("Padding to next section seems large ({}bytes), are the correct linker sections being used? Current: 0x{:08X}, Next: 0x{:08X}", padding, section.address(), next.address());
            }
            padding
        } else if data_data.len() % 4 != 0 {
            4 - (data_data.len() % 4)
        } else {
            0
        };
        data_data.extend(iter::repeat(b'\0').take(padding));
        data.extend(&data_data);
    }

    (data_start, data)
}
