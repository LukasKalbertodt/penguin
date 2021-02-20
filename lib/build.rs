use std::{error::Error, path::{Path, PathBuf}, process::Command};


// This build script compiles the Typescript code.
fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/browser.ts");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let infile = manifest_dir.join("src").join("browser.ts");
    let outfile = manifest_dir.join("src").join("generated").join("browser.js");

    // Cargo already just calls this script when the `.ts` file was changed.
    // However, we add this extra check to make sure we don't try to compile it
    // again if it's not necessary. This means that devs checking out the repo
    // or `cargo install`ing penguin don't need to have `tsc` installed (since
    // the generated file is checked into git).
    let need_compiling = !outfile.exists()
        || infile.metadata()?.modified()? > outfile.metadata()?.modified()?;

    if !need_compiling {
        return Ok(());
    }

    // Figure out which `tsc` to use. Prefer a locally installed one but if
    // that's not present, try a global `tsc`.
    let local_tsc = manifest_dir
        .join("node_modules")
        .join("typescript")
        .join("bin")
        .join("tsc");

    let tsc = if local_tsc.exists() {
        &local_tsc
    } else {
        Path::new("tsc")
    };

    // Run `tsc` and check the status.
    let status = Command::new(tsc)
        .current_dir(&manifest_dir)
        .arg("--pretty")
        .status();
    match status {
        Err(e) => {
            eprintln!("Error executing `tsc`.");
            if !local_tsc.exists() {
                eprintln!("You might need to run `npm install` in the `lib` folder");
            }
            Err(e)?;
        }
        Ok(status) if !status.success() => {
            Err("`tsc` reported errors.")?;
        }
        Ok(_) => {}
    }

    Ok(())
}
