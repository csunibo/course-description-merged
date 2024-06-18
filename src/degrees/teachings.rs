use eyre::eyre;
use eyre::Result;
use itertools::Itertools;
use reqwest::blocking;
use scraper::Selector;
use substring::Substring;

lazy_static::lazy_static! {
    static ref TITLE: Selector = Selector::parse("div#u-content-intro>h1").unwrap();
    static ref LANG: Selector = Selector::parse("li.language-en").unwrap();
    static ref DESC: Selector = Selector::parse("div.description-text").unwrap();
    static ref DESC_END_MARKER: std::collections::HashMap<String,String> = [
        ("Numerical Computing".to_string(), "Teaching".to_string()),
        ("History of Informatics".to_string(), "Office".to_string()),
        ("*".to_string(), "Readings".to_string())
    ]
    .into();
}

fn get_eng_url(url: &str) -> Result<String> {
    if url.is_empty() {
        return Ok("".to_string());
    }
    let res = blocking::get(url)?.text()?;
    let document = scraper::Html::parse_document(&res);
    let mut link_ite = document.select(&LANG).map(|x| x.inner_html());
    link_ite.next().ok_or(eyre!("Cannot get english url"))
}

pub fn get_desc_teaching_page(url: &str) -> Result<String> {
    let eng_url_temp = match get_eng_url(url) {
        Ok(url) => url,
        Err(e) => return Err(eyre!(e.to_string())), // interniships, thesis...
    };
    let start = eng_url_temp.find("http").unwrap_or(0);
    let tmp = eng_url_temp.substring(start, eng_url_temp.len());
    let end = tmp.find('\"').unwrap_or(0);
    let teaching_url = tmp.substring(0, end);
    let eng_page = blocking::get(teaching_url)?.text()?;
    let document = scraper::Html::parse_document(&eng_page);
    let teaching_title = document
        .select(&TITLE)
        .next()
        .ok_or(eyre!("Cannot parse teaching title"))?
        .text()
        .join("");
    let full_description = document
        .select(&DESC)
        .next()
        .ok_or(eyre!("Cannot parse teaching description"))?
        .text()
        .join("");
    let i = full_description
        .find("Learning outcomes")
        .unwrap_or(full_description.len());
    let mut f: Option<usize> = DESC_END_MARKER
        .get("*")
        .and_then(|marker| full_description.find(marker));
    for (pattern, marker) in DESC_END_MARKER.iter() {
        if teaching_title.contains(pattern.as_str()) {
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
        teaching_url,
        teaching_title.as_str(),
        filtered_description.trim()
    ))
}
