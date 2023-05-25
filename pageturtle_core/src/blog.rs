use std::{path::{PathBuf, Path}, borrow::Borrow};

use crate::utils::date;
use chrono::{DateTime, Utc, Datelike, Timelike};
use comrak::{nodes::AstNode, ComrakOptions, ComrakPlugins, Arena};
use serde::Deserialize;
use slug::slugify;


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

#[derive(Deserialize)]
pub struct Link {
    pub name: String,
    pub href: String,
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

impl BlogConfiguration {
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        let config = toml::from_str(content)?;
        Ok(config)
    }
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
    pub fn format_date(&self) -> String {
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

#[derive(Debug)]
pub struct BlogPost<'a> {
    pub metadata: BlogPostMetadata,
    pub raw_content: String,
    pub ast: &'a AstNode<'a>,
}

#[derive(Debug)]
/// Error that can happen when parsing a post and compiling it to HTML
pub struct CompilePostError {
    pub line: u32,
    pub column: u32,
    pub message: String,
}

#[derive(Debug)]
pub struct PublishableBlogPost<'a> {
    pub post: &'a BlogPost<'a>,
    pub filename: PathBuf,
    pub description: String,
    pub rendered_html: String,
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
            // TODO: line and column error messages
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

fn parse_frontmatter<'a>(ast: &'a AstNode<'a>) -> Result<BlogPostMetadata, String> {
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
        Some(s) => {
            let unquoted = s.replace("---", "");
            match serde_yaml::from_str::<BlogPostMetadata>(&unquoted) {
                Ok(settings) => Ok(settings),
                Err(e) => Err(e.to_string()),
            }
        },
        None => Err("could not find frontmatter section in file".to_owned()),
    }
}