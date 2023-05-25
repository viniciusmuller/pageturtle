use crate::blog::{BlogConfiguration, PublishableBlogPost};
use chrono::{Datelike, NaiveDate, Utc};

#[derive(Debug)]
pub struct FeedEntry<'a> {
    pub id: String,
    pub title: &'a str,
    pub content: &'a str,
    pub author: &'a str,
    /// RFC3339 formatted date
    pub updated: String,
    pub link: String,
}

#[derive(Debug)]
pub struct Feed<'a> {
    pub title: &'a str,
    pub link: &'a str,
    pub author: &'a str,
    /// RFC3339 formatted date
    pub updated: String,
    pub entries: Vec<FeedEntry<'a>>,
}

pub fn build_feed<'a>(
    posts: &'a [PublishableBlogPost<'a>],
    config: &'a BlogConfiguration,
) -> Feed<'a> {
    let entries = posts.iter().map(|p| to_entry(p, config)).collect();

    Feed {
        author: &config.author,
        title: &config.blog_title,
        link: &config.base_url,
        updated: rfc3339_date(Utc::now().naive_utc().date()),
        entries,
    }
}

fn to_entry<'a>(post: &'a PublishableBlogPost<'a>, config: &'a BlogConfiguration) -> FeedEntry<'a> {
    let filename = post.filename.to_str().unwrap();
    let url = format!("{}/{}", config.base_url, filename);

    FeedEntry {
        id: url.to_owned(),
        title: &post.post.metadata.title,
        author: &config.author, // TODO: use post author if set
        content: &post.rendered_html,
        updated: rfc3339_date(post.post.metadata.date),
        link: url,
    }
}

fn rfc3339_date(date: NaiveDate) -> String {
    format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
        date.year(),
        date.month(),
        date.day(),
        0,
        0,
        0
    )
}
