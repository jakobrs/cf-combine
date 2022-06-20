use std::{
    borrow::Cow,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;

fn resolve<'a>(
    dir: &'a Path,
    this_module_name: &str,
    new_module_name: &str,
) -> Result<Option<(PathBuf, Cow<'a, Path>)>> {
    let mut path: PathBuf = dir.to_owned();

    path.push(this_module_name);
    if path.is_dir() {
        path.push(&format!("{new_module_name}.rs"));

        if path.is_file() {
            let file_path = path.clone();
            let mut dir_path = path;
            dir_path.pop();

            return Ok(Some((file_path, Cow::Owned(dir_path))));
        }

        path.pop();
    }
    path.pop();

    path.push(new_module_name);
    if path.is_dir() {
        path.push("mod.rs");

        if path.is_file() {
            let file_path = path.clone();
            let mut dir_path = path;
            dir_path.pop();

            return Ok(Some((file_path, Cow::Owned(dir_path))));
        }

        path.pop();
    }
    path.pop();

    path.push(&format!("{new_module_name}.rs"));

    if path.is_file() {
        return Ok(Some((path, Cow::Borrowed(dir))));
    }

    Ok(None)
}

struct RegexContext {
    re: Regex,
}

impl RegexContext {
    fn new() -> Result<Self> {
        Ok(Self {
            re: Regex::new(r"^(pub(?:\([a-z:]*\))? )?mod ([a-z_]*);$").context("Parsing regex")?,
        })
    }

    fn process(&self, dir: &Path, this_module_name: &str, data: &str) -> Result<String> {
        let mut output = String::new();

        for line in data.lines() {
            if let Some(matches) = self.re.captures(line) {
                let new_module_name = &matches[2];

                let (file_path, dir_path) = resolve(dir, this_module_name, new_module_name)
                    .context("Resolving module path")?
                    .context("Unable to resolve module path")?;

                let mut mod_contents = String::new();
                let mut file = File::open(&file_path).context("Opening module")?;
                file.read_to_string(&mut mod_contents)
                    .context("Reading module")?;

                let recursive_output = self.process(&dir_path, new_module_name, &mod_contents)?;

                if let Some(modifiers) = matches.get(1) {
                    output += modifiers.as_str();
                }
                output += "mod ";
                output += new_module_name;
                output += " {\n";
                output += &recursive_output;
                output += "}\n";
            } else {
                output += &line;
                output += "\n";
            }
        }

        Ok(output)
    }
}

#[derive(Parser)]
/// Combines multiple Rust source files into one
struct Args {
    /// The file
    file: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut input = String::new();
    File::open(&args.file)
        .context("Opening file")?
        .read_to_string(&mut input)
        .context("Reading file")?;

    let mut dir = args.file;
    dir.pop();

    let ctx = RegexContext::new()?;
    let output = ctx.process(&dir, "lib", &input)?;

    println!("{output}");

    Ok(())
}
