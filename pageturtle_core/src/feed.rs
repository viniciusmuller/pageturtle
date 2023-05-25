use chrono::{Utc, DateTime};
use serde::Serialize;
use serde_xml_rs::to_string;
use crate::{utils::date, blog::{PublishableBlogPost, BlogConfiguration}};

#[derive(Serialize)]
pub struct Author {
    name: String,
    email: String
}

#[derive(Serialize)]
struct Content {
    #[serde(rename = "type")]
    typ: String,
    body: String
}

#[derive(Serialize)]
struct Link {
    href: String,
    rel: String
}

impl Link {
    fn new(href: String) -> Self {
        Self {
            href,
            rel: "alternate".to_owned()
        }
    }
} 

#[derive(Serialize)]
struct Entry {
    id: String,
    title: String,
    content: Content,
    link: Link
}

#[derive(Serialize)]
struct Feed {
    title: String,
    link: String,
    #[serde(with = "date")]
    updated: DateTime<Utc>,
    entries: Vec<Entry>
}

pub fn build_feed<'a>(posts: &'a Vec<PublishableBlogPost<'a>>, config: &'a BlogConfiguration) -> String {
    let feed = Feed {
        title: todo!(),
        link: todo!(),
        updated: todo!(),
        entries: todo!(),
    };

    to_string(&feed).unwrap()
}
