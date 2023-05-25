use chrono::{Utc, DateTime};
use crate::{blog::{PublishableBlogPost, BlogConfiguration}};

// TODO: author
#[derive(Debug)]
pub struct Author {
    name: String,
    email: String
}

#[derive(Debug)]
pub struct FeedEntry<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub content: &'a str,
    pub updated: DateTime<Utc>,
    pub link: String
}

#[derive(Debug)]
pub struct Feed<'a> {
    pub title: &'a str,
    pub link: &'a str,
    pub updated: DateTime<Utc>,
    pub entries: Vec<FeedEntry<'a>>
}

pub fn build_feed<'a>(posts: &'a Vec<PublishableBlogPost<'a>>, config: &'a BlogConfiguration) -> Feed<'a> {
    let entries = posts
        .iter()
        .map(|p| to_entry(&p, &config))
        .collect();

    Feed {
        title: &config.blog_title,
        link: &config.base_url,
        updated: Utc::now(),
        entries
    }
}

fn to_entry<'a>(post: &'a PublishableBlogPost<'a>, config: &'a BlogConfiguration) -> FeedEntry<'a> {
    let filename = post.filename.to_str().unwrap();

    FeedEntry { 
        id: filename,
        title: &post.post.metadata.title,
        content: &post.rendered_html,
        updated: post.post.metadata.date,
        link: format!("{}/{}",config.base_url, filename),
    }
}
