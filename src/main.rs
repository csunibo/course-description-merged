use std::{collections::HashMap, fmt::Write};

use eyre::{eyre, Result};
use itertools::Itertools;
use lazy_static::lazy_static;
use scraper::Selector;
use substring::Substring;

lazy_static! {
    static ref TABLE: Selector = scraper::Selector::parse("td.title").unwrap();
    static ref TITLE: Selector = scraper::Selector::parse("div#u-content-intro>div>h1").unwrap();
    static ref LANG: Selector = scraper::Selector::parse("li.language-en").unwrap();
    static ref DESC: Selector = scraper::Selector::parse("div.description-text").unwrap();
    static ref DESC_END_MARKER: HashMap<String, String> = [
        ("Numerical Computing".to_string(), "Teaching".to_string()),
        ("History of Informatics".to_string(), "Office".to_string()),
        ("*".to_string(), "Readings".to_string())
    ]
    .into();
    static ref MISSING_TRANSLATIONS: HashMap<String, String> = [
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
            "Course contents".to_string(),
            "=== Course contents".to_string()
        )
    ]
    .into();
}

fn get_desc_course_page(url: &str) -> Result<String> {
    let eng_url_temp = get_eng_url(url)?;

    // escludo l'idoneità di inglese e i corsi che non hanno una pagina (prova finale, tirocinio, corsi non attivi...)
    if eng_url_temp.contains("26338") || eng_url_temp.is_empty() {
        return Ok("".to_string());
    }

    let start = eng_url_temp.find("http").unwrap_or(0);
    let tmp = eng_url_temp.substring(start, eng_url_temp.len());
    let end = tmp.find('\"').unwrap_or(0);
    let course_url = tmp.substring(0, end);

    let eng_page = reqwest::blocking::get(course_url)?.text()?;
    let document = scraper::Html::parse_document(&eng_page);

    let course_title = document
        .select(&TITLE)
        .next()
        .ok_or(eyre!("Cannot parse course title"))?
        .text()
        .join("");

    let full_description = document
        .select(&DESC)
        .next()
        .ok_or(eyre!("Cannot parse course description"))?
        .text()
        .join("");

    let i = full_description
        .find("Learning outcomes")
        .unwrap_or(full_description.len());

    let mut f: Option<usize> = DESC_END_MARKER
        .get("*")
        .and_then(|marker| full_description.find(marker));

    for (pattern, marker) in DESC_END_MARKER.iter() {
        if course_title.contains(pattern.as_str()) {
            f = full_description
                .find(marker)
                .or(Some(full_description.len()));
            break;
        }
    }

    let filtered_description = full_description
        .substring(
            i,
            f.ok_or(eyre!(
                "No description end marker defined for this page content"
            ))? - 2,
        )
        .split('\n')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .join("\n\n");

    Ok(format!(
        "\n== {}[{}]\n{}",
        course_url,
        course_title.as_str(),
        filtered_description.trim()
    ))
}

fn get_eng_url(url: &str) -> Result<String> {
    if url.is_empty() {
        return Ok("".to_string());
    }

    let res = reqwest::blocking::get(url)?.text()?;
    let document = scraper::Html::parse_document(&res);
    let mut link_ite = document.select(&LANG).map(|x| x.inner_html());

    link_ite.next().ok_or(eyre!("Cannot get english url"))
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let res = reqwest::blocking::get(
        "https://corsi.unibo.it/laurea/informatica/insegnamenti/piano/2022/8009/000/000/2022",
    )?
    .text()?;

    let document = scraper::Html::parse_document(&res);
    let title_list = document.select(&TABLE);

    let mut buf = String::new();

    for item in title_list {
        let mut entry_doc = "".to_string();

        let a_el = item
            .children()
            .filter_map(|f| f.value().as_element())
            .find(|r| r.name() == "a")
            .map(|a_el| a_el.attr("href"))
            .flatten();

        let course_url = match a_el {
            Some(a) => a,
            None => {
                eprintln!("Cannot parse an element: {}", item.text().join("").trim());
                continue;
            }
        };

        println!("Visiting {}", course_url);
        let course_desc = get_desc_course_page(course_url)?;
        entry_doc += "\n";
        entry_doc += course_desc.as_str();

        for (source, replacement) in MISSING_TRANSLATIONS.iter() {
            entry_doc = entry_doc.replace(source, replacement);
        }

        buf.write_str(&entry_doc)?;
        println!("Course fetched!");
    }

    std::fs::write("courses.adoc", buf)?;
    Ok(())
}
