#![feature(iter_intersperse)]
use std::collections::HashMap;

use eyre::{eyre, Result};
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
        ("FONDAMENTI DI".to_string(), "".to_string()),
        (
            "Learning outcomes".to_string(),
            "### Learning outcomes".to_string()
        ),
        (
            "Course contents".to_string(),
            "### Course contents".to_string()
        )
    ]
    .into();
}

async fn get_desc_course_page(url: &str) -> Result<String> {
    let eng_url_temp = get_eng_url(url).await?;

    // escludo l'idoneità di inglese e i corsi che non hanno una pagina (prova finale, tirocinio, corsi non attivi...)
    if eng_url_temp.contains("26338") || eng_url_temp.is_empty() {
        return Ok("".to_string());
    }

    let start = eng_url_temp.find("http").unwrap_or(0);
    let tmp = eng_url_temp.substring(start, eng_url_temp.len());
    let end = tmp.find('\"').unwrap_or(0);
    let course_url = tmp.substring(0, end);
    // println!("{}", course_url);
    let res = reqwest::get(course_url).await?.text().await?;
    let document = scraper::Html::parse_document(&res);

    let codetitle = document
        .select(&TITLE)
        .next()
        .ok_or(eyre!("Cannot parse title"))?
        .text()
        .map(|t| t.to_string())
        .intersperse("".to_string())
        .collect::<String>();

    let desc = document
        .select(&DESC)
        .next()
        .ok_or(eyre!("Cannot parse description"))?
        .text()
        .map(|t| t.to_string())
        .intersperse("".to_string())
        .collect::<String>();

    let i = desc.find("Learning outcomes").unwrap_or(desc.len());
    let mut f: Option<usize> = DESC_END_MARKER
        .get("*")
        .and_then(|marker| desc.find(marker));
    for (pattern, marker) in DESC_END_MARKER.iter() {
        if codetitle.contains(pattern.as_str()) {
            f = desc.find(marker).or(Some(desc.len()));
            break;
        }
    }
    let final_desc = desc
        .substring(
            i,
            f.ok_or(eyre!(
                "No description end marker defined for this page content"
            ))? - 2,
        )
        .split('\n')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .intersperse("\n\n\n")
        .collect::<String>();

    Ok("\n## ".to_string() + codetitle.as_str() + "\n" + final_desc.trim())
}

async fn get_eng_url(url: &str) -> Result<String> {
    if url.is_empty() {
        return Ok("".to_string());
    }

    let res = reqwest::get(url).await?.text().await?;
    let document = scraper::Html::parse_document(&res);
    let mut link_ite = document.select(&LANG).map(|x| x.inner_html());

    link_ite.next().ok_or(eyre!("Cannot get english url"))
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let res = reqwest::get(
        "https://corsi.unibo.it/laurea/informatica/insegnamenti/piano/2022/8009/000/000/2022",
    )
    .await?
    .text()
    .await?;

    let document = scraper::Html::parse_document(&res);
    let title_list = document.select(&TABLE).map(|x| x.inner_html());

    let mut c = 1;

    let mut final_document = "".to_string();
    for item in title_list {
        let start = item.find('\"').unwrap_or(0);
        let end = item.find("e\"").unwrap_or(0);
        let ita_url = item.substring(start + 1, end + 1);
        let course_desc = get_desc_course_page(ita_url).await?;
        final_document += "\n";
        final_document += course_desc.as_str();
        println!("Course {c} fetched!");
        c += 1;
    }

    for (source, replacement) in MISSING_TRANSLATIONS.iter() {
        final_document = final_document.replace(source, replacement);
    }

    std::fs::write("description.md", final_document)?;

    Ok(())
}
