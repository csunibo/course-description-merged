use degrees::degrees;
use log::error;
use std::{fmt::Write, fs};

pub mod degrees;

fn main() {
    let env = env_logger::Env::default().default_filter_or("info");
    env_logger::Builder::from_env(env).init();
    if let Err(e) = color_eyre::install() {
        error!("Eyre setup: {e}");
        return;
    };
    let output_dir = std::path::Path::new("output");
    if !output_dir.exists() {
        if let Err(e) = fs::create_dir(output_dir) {
            error!("Output dir creation: {e}");
            return;
        };
    }
    let mut index = "= Index\n\n".to_owned();
    if let Some(deg) = degrees() {
        for d in deg {
            degrees::analyze_degree(&d, &output_dir);
            if let Err(e) = writeln!(index, "* xref:degree-{}.adoc[{}]", d.slug, d.name) {
                error!("Could not append {}: {}", d.name, e);
            };
        }
    } else {
        error!("Could not load degrees");
        return;
    }
    if let Err(e) = fs::write(output_dir.join("index.adoc"), index) {
        error!("Could not write index: {e}")
    };
}
