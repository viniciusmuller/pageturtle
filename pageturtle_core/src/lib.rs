// TODO: modularize
// TODO: move core to another crate and leave this just as a CLI app
use askama::Template; // bring trait in scope
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::Deserialize;
use slug::slugify;
use std::{
    borrow::Borrow,
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
};

use comrak::{
    arena_tree::Node,
    nodes::{Ast, AstNode},
    Arena, ComrakOptions, ComrakPlugins,
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
    // TODO: compose templates
    config: &'a BlogConfiguration,
    post: &'a PublishableBlogPost<'a>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    // TODO: compose templates
    config: &'a BlogConfiguration,
    posts: &'a Vec<PublishableBlogPost<'a>>,
}

#[derive(Deserialize)]
pub struct Link {
    name: String,
    href: String,
}

#[derive(Deserialize)]
pub struct BlogConfiguration {
    pub blog_title: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default = "default_true")]
    pub enable_rss: bool,
    pub extra_links_start: Option<Vec<Link>>,
    pub extra_links_end: Option<Vec<Link>>,
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

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct BlogPostMetadata {
    pub title: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    #[serde(with = "date")]
    pub date: DateTime<Utc>,
    pub tags: Option<Vec<String>>,
}

impl BlogPostMetadata {
    fn format_date(&self) -> String {
        let date = self.date;
        let (_is_common_era, year) = date.year_ce();
        let hour = date.hour();

        format!(
            "{}, {}/{:02}/{:02}, {:02}:{:02}",
            date.weekday(),
            year,
            date.month(),
            date.day(),
            hour,
            date.minute(),
        )
    }
}

pub struct PostCompiler<'a> {
    arena: Arena<AstNode<'a>>,
    options: &'a ComrakOptions,
    plugins: &'a ComrakPlugins<'a>,
}

pub fn parse_config(content: &str) -> Result<BlogConfiguration, toml::de::Error> {
    let config = toml::from_str(content)?;
    Ok(config)
}

impl<'a> PostCompiler<'a> {
    pub fn new(
        arena: Arena<AstNode<'a>>,
        options: &'a ComrakOptions,
        plugins: &'a ComrakPlugins<'a>,
    ) -> PostCompiler<'a> {
        Self {
            arena,
            options,
            plugins,
        }
    }

    pub fn to_ast(&'a self, content: &str) -> &'a AstNode<'a> {
        comrak::parse_document(&self.arena, content, self.options)
    }

    pub fn ast_to_html(&'a self, ast: &'a AstNode<'a>) -> String {
        let mut output_buffer = Vec::new();
        comrak::format_html_with_plugins(ast, self.options, &mut output_buffer, self.plugins)
            .unwrap();
        String::from_utf8(output_buffer).unwrap()
    }
}

pub fn stylesheet() -> String {
    fs::read_to_string("./pageturtle_core/assets/styles.css").unwrap()
}

mod date {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer};

    const FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug)]
pub struct BlogPost<'a> {
    pub metadata: BlogPostMetadata,
    pub raw_content: String,
    pub ast: &'a Node<'a, RefCell<Ast>>,
}

#[derive(Debug)]
pub struct PublishableBlogPost<'a> {
    pub post: &'a BlogPost<'a>,
    pub filename: PathBuf,
    pub description: String,
    pub rendered_html: String,
}

#[derive(Debug)]
/// Error that can happen when parsing a post and compiling it to HTML
pub struct CompilePostError {
    pub line: u32,
    pub column: u32,
    pub message: String,
}

pub fn prepare_for_publish<'a>(
    p: &'a BlogPost<'a>,
    compiler: &'a PostCompiler<'a>,
) -> PublishableBlogPost<'a> {
    let rendered_html = compiler.ast_to_html(p.ast);

    let metadata = &p.metadata;
    let filename = match metadata.slug {
        Some(ref s) => slugify(s),
        None => slugify(&metadata.title),
    };
    let filename = Path::new(&filename).with_extension("html");

    let description = match p.metadata.description {
        Some(ref d) => d.to_owned(),
        None => "TODO: automatically build description".to_string(),
    };

    PublishableBlogPost {
        post: p,
        filename,
        description,
        rendered_html,
    }
}

pub fn build_blog_post<'a>(
    content: &str,
    compiler: &'a PostCompiler<'a>,
) -> Result<BlogPost<'a>, CompilePostError> {
    let ast = compiler.to_ast(content);

    let metadata = match parse_frontmatter(ast) {
        Ok(settings) => settings,
        Err(msg) => {
            return Err(CompilePostError {
                message: msg,
                line: 10,
                column: 20,
            })
        }
    };

    Ok(BlogPost {
        raw_content: content.to_owned(),
        ast,
        metadata,
    })
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

fn parse_frontmatter<'a>(ast: &'a Node<'a, RefCell<Ast>>) -> Result<BlogPostMetadata, String> {
    use comrak::nodes::NodeValue::*;

    let mut frontmatter: Option<String> = None;

    for node in ast.borrow().traverse() {
        match node {
            comrak::arena_tree::NodeEdge::Start(nv) => {
                if let FrontMatter(s) = &nv.borrow().data.borrow().value {
                    frontmatter = Some(s.to_owned())
                }
            }
            comrak::arena_tree::NodeEdge::End(_nv) => continue,
        }
    }

    match frontmatter {
        Some(s) => parse_frontmatter_yaml(&s),
        None => Err("could not find frontmatter section in file".to_owned()),
    }
}

fn parse_frontmatter_yaml(s: &str) -> Result<BlogPostMetadata, String> {
    let unquoted = s.replace("---", "");
    match serde_yaml::from_str::<BlogPostMetadata>(&unquoted) {
        Ok(settings) => Ok(settings),
        Err(e) => Err(e.to_string()),
    }
}
