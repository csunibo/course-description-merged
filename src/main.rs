use std::{fmt::Write, fs};

pub mod degrees;

fn main() -> Result<(), eyre::Report> {
    color_eyre::install()?;
    let output_dir = std::path::Path::new("output");
    if !output_dir.exists() {
        fs::create_dir(output_dir)?;
    }
    let mut index = "= Index\n\n".to_owned();
    for degrees::Degree { slug, name, url } in degrees::degrees() {
        degrees::analyze_degree(
            &name,
            &output_dir.join(format!("degree-{}.adoc", slug)),
            &url,
        )?;
        write!(index, "* xref:degree-{}.adoc[{}]\n", slug, name)?;
    }
    fs::write(output_dir.join("index.adoc"), index)?;
    Ok(())
}
