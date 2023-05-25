use askama::Template;

use crate::blog::{BlogConfiguration, PublishableBlogPost, BlogPost};

#[derive(Template)]
#[template(path = "tags.html")]
struct TagsTemplate<'a> {
    config: &'a BlogConfiguration,
    tags: Vec<&'a String>,
}

#[derive(Template)]
#[template(path = "post.html", escape = "none")]
struct PostTemplate<'a> {
    config: &'a BlogConfiguration,
    post: &'a PublishableBlogPost<'a>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    config: &'a BlogConfiguration,
    posts: &'a Vec<PublishableBlogPost<'a>>,
}

pub fn render_tags_page<'a>(posts: &Vec<BlogPost<'a>>, config: &BlogConfiguration) -> String {
    let mut all_tags = Vec::new();

    for post in posts {
        match post.metadata.tags {
            Some(ref post_tags) => all_tags.extend(post_tags),
            None => (),
        }
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
    PostTemplate { post, config }.render().unwrap()
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
