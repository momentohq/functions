#[allow(clippy::expect_used)]
fn main() {
    // Pull the workspace wit files into OUT_DIR so they can be used as part of the
    // build during publish.
    let directory_to_sync = "../wit";

    match std::process::Command::new("cp")
        .arg("-r")
        .arg(directory_to_sync)
        .arg(std::env::var("OUT_DIR").expect("OUT_DIR must exist"))
        .status()
    {
        Ok(_) => (),
        Err(e) => {
            println!("cargo:warning=Failed to sync {directory_to_sync}: {e:?}");
        }
    }

    println!("cargo:rerun-if-changed={directory_to_sync}");
}
