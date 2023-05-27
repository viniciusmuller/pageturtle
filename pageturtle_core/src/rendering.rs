use askama::Template;
use comrak::adapters::{HeadingAdapter, HeadingMeta};

use crate::{
    blog::{BlogConfiguration, BlogPost, PublishableBlogPost, TableOfContents, TableOfContentsEntry},
    feed::Feed,
};

#[derive(Template)]
#[template(path = "tags.html")]
struct TagsTemplate<'a> {
    config: &'a BlogConfiguration,
    tags: Vec<&'a String>,
}

#[derive(Template)]
#[template(path = "post.html", escape = "none")]
struct PostTemplate<'a> {
    toc: Option<TocTemplate>,
    authors: String,
    config: &'a BlogConfiguration,
    post: &'a PublishableBlogPost<'a>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    config: &'a BlogConfiguration,
    posts: &'a Vec<PublishableBlogPost<'a>>,
}

#[derive(Template)]
#[template(path = "toc-entry.html", escape = "none")]
struct TocEntryTemplate {
    title: String,
    anchor: String,
    children: Vec<TocEntryTemplate>,
}

impl<'a> TocEntryTemplate {
    fn from_toc_entry(entry: &TableOfContentsEntry) -> TocEntryTemplate {
        let children = entry.
                children.
                iter().
                map(|e| Self::from_toc_entry(e))
                .collect::<Vec<TocEntryTemplate>>();

        TocEntryTemplate { 
            children,
            title: entry.title.clone(),
            anchor: entry.anchor.clone(),
        }
    }
}

#[derive(Template)]
#[template(path = "toc.html", escape = "none")]
struct TocTemplate {
    entries: Vec<TocEntryTemplate>,
}

impl<'a> TocTemplate {
    pub fn from_toc(toc: &TableOfContents) -> TocTemplate {
        let mut templates = Vec::new();

        for entry in &toc.entries {
            templates.push(TocEntryTemplate::from_toc_entry(entry));
        }

        TocTemplate { entries: templates }
    }
}

#[derive(Template)]
#[template(path = "atom.xml")]
struct FeedTemplate<'a> {
    feed: &'a Feed<'a>,
}

pub fn render_tags_page(posts: &Vec<BlogPost<'_>>, config: &BlogConfiguration) -> String {
    let mut all_tags = Vec::new();

    for post in posts {
        all_tags.extend(&post.metadata.tags);
    }

    TagsTemplate {
        config,
        tags: all_tags,
    }
    .render()
    .unwrap()
}

pub fn render_post_page<'a>(
    post: &'a PublishableBlogPost<'a>,
    config: &'a BlogConfiguration,
) -> String {
    let authors = post
        .post
        .metadata
        .authors
        .as_ref()
        .map(|v| v.join(", "))
        .unwrap_or(config.author.clone());

    let toc = if post.post.metadata.table_of_contents {
        Some(TocTemplate::from_toc(&post.post.toc))
    } else {
        None
    };

    PostTemplate {
        authors,
        post,
        config,
        toc,
    }
    .render()
    .unwrap()
}

pub fn render_index<'a>(
    posts: &'a Vec<PublishableBlogPost<'a>>,
    config: &'a BlogConfiguration,
) -> String {
    IndexTemplate { posts, config }.render().unwrap()
}

pub fn stylesheet() -> String {
    let styles_bytes = include_bytes!("../assets/styles.css");
    String::from_utf8(styles_bytes.to_vec()).unwrap()
}

pub fn render_feed<'a>(feed: &'a Feed<'a>) -> String {
    FeedTemplate { feed }.render().unwrap()
}
