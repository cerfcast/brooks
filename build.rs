#[cfg(feature = "nginx")]
use git2::Repository;
#[cfg(feature = "nginx")]
use std::path::{Path, PathBuf};
#[cfg(feature = "nginx")]
use std::{env, path};
#[cfg(feature = "caddy")]
use std::{env, path::PathBuf};

#[cfg(feature = "caddy")]
fn main() {
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("src/integrations/caddy/caddy.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

#[cfg(feature = "nginx")]
fn main() {
    // First, make sure that the user has nginx source code.
    let nginx_path = Path::new("./integrations/nginx/nginx");
    if !nginx_path.exists() {
        let nginx_install_path = path::absolute(Path::new("./integrations/nginx/install"))
            .expect("Could not get the nginx install path");
        let nginx_module_path = path::absolute("./integrations/nginx/module")
            .expect("Could not get the nginx module path");
        let url = "https://github.com/nginx/nginx.git";
        Repository::clone(url, nginx_path).expect("Could not clone nginx source code repository.");

        println!(
            "{:?}",
            std::process::Command::new("./auto/configure")
                .current_dir(nginx_path)
                .args([
                    "--with-debug",
                    &format!(
                        "--add-module={}",
                        nginx_module_path
                            .to_str()
                            .expect("could not convert the nginx module path to a string")
                    ),
                    &format!(
                        "--prefix={}",
                        nginx_install_path
                            .to_str()
                            .expect("Could not convert the nginx install path to a string")
                    ),
                ])
                .output()
                .expect("Could not configure nginx")
        );
    }

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        .clang_args([
            "-F./integrations/nginx/nginx/src/core/",
            "-F./integrations/nginx/nginx/src/http/",
            "-F./integrations/nginx/nginx/src/http/modules/",
            "-F./integrations/nginx/nginx/src/event/",
            "-F./integrations/nginx/nginx/src/event/modules/",
            "-F./integrations/nginx/nginx/objs/",
            "-F./integrations/nginx/nginx/src/os/unix/",
        ])
        // The input header we would like to generate
        // bindings for.
        .header("src/integrations/nginx/nginx.h")
        .allowlist_item("ngx_http_request_s")
        .allowlist_item("ngx_http_headers_in_t")
        .allowlist_item("ngx_http_headers_out_t")
        .allowlist_item("ngx_pcalloc")
        .allowlist_item("ngx_list_push")
        .allowlist_item("ngx_log.*")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

#[cfg(not(any(feature = "nginx", feature = "caddy")))]
fn main() {}
