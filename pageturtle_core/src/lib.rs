// TODO: modularize
// TODO: move core to another crate and leave this just as a CLI app
use askama::Template; // bring trait in scope
use chrono::{DateTime, Utc};
use serde::Deserialize;
use slug::slugify;
use std::{
    borrow::Borrow,
    cell::RefCell,
    path::{Path, PathBuf}, fs,
};


use comrak::{
    arena_tree::Node,
    nodes::{Ast, AstNode},
    Arena, ComrakOptions, ComrakPlugins,
};

#[derive(Template)]
#[template(path = "post.html", escape = "none")]
struct PostTemplate<'a> {
    // TODO: compose templates
    base_url: &'a str,
    title: &'a str,
    content: &'a str,
}

#[derive(Template)]
#[template(path = "index.html", escape = "none")]
struct IndexTemplate<'a> {
    // TODO: compose templates
    base_url: &'a str,
    title: &'a str,
    posts: &'a Vec<PublishableBlogPost<'a>>
}

#[derive(Debug, Deserialize)]
pub struct BlogPostMetadata {
    title: String,
    slug: Option<String>,
    description: Option<String>,
    #[serde(with = "date")]
    date: DateTime<Utc>,
    tags: Option<Vec<String>>,
}

pub struct PostCompiler<'a> {
    arena: Arena<AstNode<'a>>,
    options: &'a ComrakOptions,
    plugins: &'a ComrakPlugins<'a>,
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
    fs::read_to_string("./pageturtle_core/templates/styles.css").unwrap()
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
    let html_document = render_post_page(p, compiler);
    let metadata = &p.metadata;
    let filename = match metadata.slug {
        Some(ref s) => slugify(s),
        None => slugify(&metadata.title),
    };
    let filename = Path::new(&filename).with_extension("html");

    let description = match p.metadata.description {
        Some(ref d) => d.to_owned(),
        None => "oh no".to_string()
    };

    PublishableBlogPost {
        post: p,
        filename,
        description,
        rendered_html: html_document,
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

fn render_post_page<'a>(p: &'a BlogPost<'a>, compiler: &'a PostCompiler<'a>) -> String {
    let rendered_html = compiler.ast_to_html(p.ast);
    let base_url = "/home/vini/projects/rust/personal/pageturtle/dist";

    PostTemplate {
        title: &p.metadata.title,
        content: &rendered_html,
        base_url
    }
    .render()
    .unwrap()
}

pub fn render_index<'a>(posts: &'a Vec<PublishableBlogPost<'a>>) -> String {
    // TODO: get summary from posts (maybe first AST text nodes) ?
    // let iter = posts.into_iter();

    let base_url = "/home/vini/projects/rust/personal/pageturtle/dist";
    IndexTemplate { posts, title: "Welcome to the blog", base_url }
    .render()
    .unwrap()
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
