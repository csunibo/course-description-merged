use std::{fmt::Write, fs, path::Path};

use itertools::Itertools;
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

fn parse_degree(predegree: &Predegree, academic_year: u32) -> Degree {
    let Predegree { name, id, code } = predegree;
    let unibo_slug = name.replace("", "");
    Degree {
        name: name.to_string(),
        slug: id.to_string(),
        url: format!("https://corsi.unibo.it/laurea/{unibo_slug}/insegnamenti/piano/{academic_year}/{code}/000/{academic_year}")
    }
}

fn to_degrees(predegrees: Vec<Predegree>) -> Vec<Degree> {
    let academic_year = year::current_academic_year();
    predegrees
        .iter()
        .map(|predegree| parse_degree(predegree, academic_year))
        .collect()
}

pub fn analyze_degree(
    degree_name: &str,
    output_file: &Path,
    teachings_url: &str,
) -> eyre::Result<(), eyre::ErrReport> {
    let res = reqwest::blocking::get(teachings_url)?.text()?;
    let document = scraper::Html::parse_document(&res);
    let title_list = document.select(&TABLE);
    let mut buf = format!("= {degree_name}\n\n");
    for item in title_list {
        let mut entry_doc = "".to_string();
        let a_el = item
            .children()
            .filter_map(|f| f.value().as_element())
            .find(|r| r.name() == "a")
            .map(|a_el| a_el.attr("href"))
            .flatten();
        let teaching_url = match a_el {
            Some(a) => a,
            None => {
                eprintln!("Cannot parse an element: {}", item.text().join("").trim());
                continue;
            }
        };
        print!("Visiting {}", teaching_url);
        let teaching_desc = match teachings::get_desc_teaching_page(teaching_url) {
            Ok(desc) => desc,
            Err(e) => {
                eprintln!("Cannot get teaching description: {}", e);
                continue;
            }
        };
        entry_doc += "\n";
        entry_doc += teaching_desc.as_str();
        for (source, replacement) in MISSING_TRANSLATIONS.iter() {
            entry_doc = entry_doc.replace(source, replacement);
        }
        buf.write_str(&entry_doc)?;
        println!("\t✓");
    }
    fs::write(output_file, buf)?;
    Ok(())
}

pub fn degrees() -> Vec<Degree> {
    let file = match fs::File::open(DEGREES_PATH) {
        Ok(file) => file,
        Err(error) => panic!("Reading {DEGREES_PATH:?}: {error:?}"),
    };
    let json: Vec<Predegree> = match serde_json::from_reader(file) {
        Ok(json) => json,
        Err(error) => panic!("Parsing {DEGREES_PATH}: {error:?}"),
    };
    to_degrees(json)
}
