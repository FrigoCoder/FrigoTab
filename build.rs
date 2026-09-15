use std::env;

fn main() {
    // Build scripts run for every target, including host-only checks.  The
    // resource compiler is meaningful only when the actual target is Windows.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=resources/Program.ico");
    println!("cargo:rerun-if-changed=resources/app.manifest");

    let mut resources = winresource::WindowsResource::new();
    resources
        .set_icon("resources/Program.ico")
        .set_manifest_file("resources/app.manifest")
        .compile()
        .expect("failed to embed FrigoTab's icon and application manifest");
}
