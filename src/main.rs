#![feature(iter_intersperse)]
use eyre::{eyre, Result};
use lazy_static::lazy_static;
use scraper::Selector;
use substring::Substring;

lazy_static! {
    static ref TABLE: Selector = scraper::Selector::parse("td.title").unwrap();
    static ref TITLE: Selector = scraper::Selector::parse("div#u-content-intro>div>h1").unwrap();
    static ref LANG: Selector = scraper::Selector::parse("li.language-en").unwrap();
    static ref DESC: Selector = scraper::Selector::parse("div.description-text").unwrap();
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
    let f: usize;
    if codetitle.contains("Numerical Computing") {
        f = desc.find("Teaching").unwrap_or(desc.len());
    } else if codetitle.contains("History of Informatics") {
        f = desc.find("Office").unwrap_or(desc.len());
    } else {
        f = desc.find("Readings").unwrap_or(desc.len());
    }
    let final_desc = desc
        .substring(i, f - 2)
        .split("\n")
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

    let mut final_document = "# {ENTER YOUR COURSE NAME}\n## by {firstname surname}\n".to_string();
    let mut c = 1;

    for item in title_list {
        let start = item.find('\"').unwrap_or(0);
        let end = item.find("e\"").unwrap_or(0);
        let ita_url = item.substring(start + 1, end + 1);
        let course_desc = get_desc_course_page(ita_url).await?;
        final_document = final_document + "\n" + course_desc.as_str();
        println!("Course {} fetched!", c);
        c += 1;
    }

    final_document = final_document
        .replace("BASI DI DATI", "DATABASES")
        .replace("FONDAMENTI DI", "")
        .replace("Learning outcomes", "### Learning outcomes")
        .replace("Course contents", "### Course contents");

    std::fs::write("description.md", final_document)?;

    Ok(())
}
