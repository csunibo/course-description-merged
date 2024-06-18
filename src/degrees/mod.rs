use std::{fmt::Write, fs, path::Path};

use itertools::Itertools;
use log::{error, info, warn};
use scraper::Selector;

pub mod teachings;
pub mod year;

lazy_static::lazy_static! {
    static ref TABLE: Selector = Selector::parse("td.title").unwrap();
    static ref MISSING_TRANSLATIONS: std::collections::HashMap<String, String> = [
        ("BASI DI DATI".to_string(), "DATABASES".to_string()),
        (
            "INTRODUZIONE ALL'APPRENDIMENTO AUTOMATICO".to_string(),
            "Introduction to machine learning".to_string()
        ),
        ("FONDAMENTI DI".to_string(), "".to_string()),
        (
            "Learning outcomes".to_string(),
            "=== Learning outcomes".to_string()
        ),
        (
            "Teaching contents".to_string(),
            "=== Teaching contents".to_string()
        )
    ]
    .into();
}

#[derive(serde::Deserialize, Debug, Clone)]
struct Predegree {
    id: String,
    name: String,
    code: String,
}

pub struct Degree {
    pub name: String,
    pub slug: String,
    pub url: String,
}

const DEGREES_PATH: &str = "config/degrees.json";

fn to_lowercase_maybe(s: String, b: bool) -> String {
    if b {
        return s.to_lowercase();
    }
    s
}

fn parse_degree(predegree: &Predegree, academic_year: u32) -> Option<Degree> {
    let Predegree { name, id, code } = predegree;
    if name.is_empty() || id.is_empty() || code.is_empty() {
        return None;
    }
    let unibo_slug = to_lowercase_maybe(
        regex::Regex::new(r"( (e|per il|in) )|Magistrale|Master")
            .unwrap()
            .replace_all(name, "")
            .to_string(),
        !code.eq("9254/000"),
    )
    // AI's slug is kebab-case
    .replace(' ', if code.eq("9063/000") { "-" } else { "" });
    let degree_type = if name.contains("Magistrale") || name.contains("Master") {
        "magistrale"
    } else {
        "laurea"
    };
    Some(Degree {
        name: name.to_string(),
        slug: id.to_string(),
        url: format!("https://corsi.unibo.it/{degree_type}/{unibo_slug}/insegnamenti/piano/{academic_year}/{code}/000/{academic_year}")
    })
}

fn to_degrees(predegrees: Vec<Predegree>) -> Vec<Degree> {
    let academic_year = year::current_academic_year();
    predegrees
        .iter()
        .filter_map(|predegree| parse_degree(predegree, academic_year))
        .collect()
}

pub fn analyze_degree(degree: &Degree, output_dir: &Path) -> Option<()> {
    let Degree { slug, name, url } = degree;
    let output_file = output_dir.join(format!("degree-{slug}.adoc"));
    info!("{name} [{url}]");
    let res = match reqwest::blocking::get(url) {
        Ok(res) => res,
        Err(e) => {
            error!("\t{e:?}");
            return None;
        }
    };
    let res2 = match res.error_for_status() {
        Ok(res2) => res2,
        Err(e) => {
            error!("\t{e:?}");
            return None;
        }
    };
    let text = match res2.text() {
        Ok(text) => text,
        Err(e) => {
            error!("\t{e:?}");
            return None;
        }
    };
    let document = scraper::Html::parse_document(&text);
    let title_list = document.select(&TABLE);
    let mut buf = format!("= {name}\n\n");
    for item in title_list {
        let mut entry_doc = "".to_string();
        let a_el = item
            .children()
            .filter_map(|f| f.value().as_element())
            .find(|r| r.name() == "a")
            .and_then(|a_el| a_el.attr("href"));
        let temp_name = item.text().join("");
        let name = temp_name.trim();
        let teaching_url = match a_el {
            Some(a) => a,
            None => {
                warn!("\tMissing link: {name}");
                continue;
            }
        };
        info!("\tVisiting {name}");
        let teaching_desc = match teachings::get_desc_teaching_page(teaching_url) {
            Ok(desc) => desc,
            Err(e) => {
                error!("\t\tCannot get description: {e:?}");
                continue;
            }
        };
        entry_doc += "\n";
        entry_doc += teaching_desc.as_str();
        for (source, replacement) in MISSING_TRANSLATIONS.iter() {
            entry_doc = entry_doc.replace(source, replacement);
        }
        if let Err(e) = buf.write_str(&entry_doc) {
            error!("\t\tCannot append: {e:?}");
            return None;
        };
    }
    if let Err(e) = fs::write(output_file, buf) {
        error!("\t\tCannot write: {e:?}");
        return None;
    };
    Some(())
}

pub fn degrees() -> Option<Vec<Degree>> {
    let file = match fs::File::open(DEGREES_PATH) {
        Ok(file) => file,
        Err(error) => {
            error!("Reading {DEGREES_PATH:?}: {error:?}");
            return None;
        }
    };
    let json: Vec<Predegree> = match serde_json::from_reader(file) {
        Ok(json) => json,
        Err(error) => {
            error!("Parsing {DEGREES_PATH}: {error:?}");
            return None;
        }
    };
    Some(to_degrees(json))
}
