use cargo_edit::shell_status;
use cargo_edit::shell_warn;
use cargo_edit::CargoResult;
use cargo_edit::{manifest_from_pkgid, LocalManifest};
use clap::Args;
use std::borrow::Cow;
use std::path::PathBuf;

/// Remove a dependency from a Cargo.toml manifest file.
#[derive(Debug, Args)]
#[command(version)]
pub struct RmArgs {
    /// Dependencies to be removed
    #[arg(value_name = "DEP_ID", required = true)]
    crates: Vec<String>,

    /// Remove as development dependency
    #[arg(long, short = 'D', conflicts_with = "build", help_heading = "Section")]
    dev: bool,

    /// Remove as build dependency
    #[arg(long, short = 'B', conflicts_with = "dev", help_heading = "Section")]
    build: bool,

    /// Remove as dependency from the given target platform
    #[arg(long, value_parser = clap::builder::NonEmptyStringValueParser::new(), help_heading = "Section")]
    target: Option<String>,

    /// Path to the manifest to remove a dependency from
    #[arg(long, value_name = "PATH")]
    manifest_path: Option<PathBuf>,

    /// Package to remove from
    #[arg(long = "package", short = 'p', value_name = "PKGID")]
    pkgid: Option<String>,

    /// Unstable (nightly-only) flags
    #[arg(short = 'Z', value_name = "FLAG", global = true, value_enum)]
    unstable_features: Vec<UnstableOptions>,

    /// Don't actually write the manifest
    #[arg(long)]
    dry_run: bool,

    /// Do not print any output in case of success
    #[arg(long, short)]
    quiet: bool,
}

impl RmArgs {
    pub fn exec(&self) -> CargoResult<()> {
        exec(self)
    }

    /// Get dependency section
    pub fn get_section(&self) -> Vec<String> {
        let section_name = if self.dev {
            "dev-dependencies"
        } else if self.build {
            "build-dependencies"
        } else {
            "dependencies"
        };

        if let Some(ref target) = self.target {
            assert!(!target.is_empty(), "Target specification may not be empty");

            vec!["target".to_owned(), target.clone(), section_name.to_owned()]
        } else {
            vec![section_name.to_owned()]
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
enum UnstableOptions {}

fn exec(args: &RmArgs) -> CargoResult<()> {
    let manifest_path = if let Some(ref pkgid) = args.pkgid {
        let pkg = manifest_from_pkgid(args.manifest_path.as_deref(), pkgid)?;
        Cow::Owned(Some(pkg.manifest_path.into_std_path_buf()))
    } else {
        Cow::Borrowed(&args.manifest_path)
    };
    let mut manifest = LocalManifest::find(manifest_path.as_deref())?;
    let deps = &args.crates;

    deps.iter()
        .map(|dep| {
            if !args.quiet {
                let section = args.get_section();
                let section = if section.len() >= 3 {
                    format!("{} for target `{}`", &section[2], &section[1])
                } else {
                    section[0].clone()
                };
                shell_status("Removing", &format!("{dep} from {section}",))?;
            }
            let result = manifest
                .remove_from_table(&args.get_section(), dep)
                .map_err(Into::into);

            // Now that we have removed the crate, if that was the last reference to that crate,
            // then we need to drop any explicitly activated features on that crate.
            manifest.gc_dep(dep);

            result
        })
        .collect::<CargoResult<Vec<_>>>()?;

    if args.dry_run {
        shell_warn("aborting rm due to dry run")?;
    } else {
        manifest.write()?;
    }

    Ok(())
}
