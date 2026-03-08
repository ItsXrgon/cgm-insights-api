//! Injects git revision at compile time for Sentry release tracking.
//! Prefers GIT_REV_SHORT env (e.g. from Docker build-arg) so Fly.io/CI can pass the SHA.

use std::process::Command;

fn main() {
    let rev = std::env::var("GIT_REV_SHORT")
        .ok()
        .filter(|s| !s.is_empty() && s != "unknown")
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short=7", "HEAD"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok()
                    } else {
                        None
                    }
                })
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_REV_SHORT={}", rev);
    println!("cargo:rerun-if-changed=.git/HEAD");
}
