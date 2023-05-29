use core::panic;
use std::{
    borrow::Borrow,
    collections::VecDeque,
    path::{Path, PathBuf},
};

use crate::utils::{date, default_empty, default_true};
use askama::filters::wordcount;
use chrono::{Datelike, NaiveDate};
use comrak::{
    adapters::{HeadingAdapter, HeadingMeta},
    nodes::{AstNode, NodeValue},
    Arena, ComrakOptions, ComrakPlugins,
};
use serde::Deserialize;
use slug::slugify;

#[derive(Debug, Clone)]
pub struct TableOfContentsEntry {
    level: u8,
    pub title: String,
    pub anchor: String,
    pub children: Vec<TableOfContentsEntry>,
}

#[derive(Debug)]
pub struct TableOfContents {
    pub entries: Vec<TableOfContentsEntry>,
}

impl<'a> TableOfContents {
    pub fn from_ast(ast: &'a AstNode<'a>) -> TableOfContents {
        let mut entries = VecDeque::new();

        for node in ast.borrow().traverse() {
            match node {
                comrak::arena_tree::NodeEdge::Start(nv) => match nv.data.borrow().value {
                    NodeValue::Heading(h) => {
                        for c in nv.children() {
                            match &c.data.borrow().value {
                                NodeValue::Text(content) => {
                                    let entry = TableOfContentsEntry {
                                        level: h.level,
                                        title: content.to_string(),
                                        anchor: slugify(content),
                                        children: Vec::new(),
                                    };

                                    entries.push_back(entry);
                                }
                                _ => continue,
                            }
                        }
                    }
                    _ => continue,
                },
                _ => continue,
            }
        }

        let mut parsed_entries = Vec::new();

        while let Some(root) = Self::build_node(&mut entries) {
            parsed_entries.push(root);
        }

        TableOfContents {
            entries: parsed_entries,
        }
    }

    fn build_node(entries: &mut VecDeque<TableOfContentsEntry>) -> Option<TableOfContentsEntry> {
        if entries.is_empty() {
            return None;
        }

        let mut root = match entries.pop_front() {
            Some(e) => e,
            None => return None,
        };

        while !entries.is_empty() {
            let mut node = entries.pop_front().unwrap();

            if node.level > root.level {
                if let Some(child) = Self::build_node(entries) {
                    if node.level >= child.level {
                        entries.push_front(child);
                        root.children.push(node);
                        return Some(root);
                    } else {
                        node.children.push(child)
                    }
                }

                root.children.push(node.to_owned());
            }

            if node.level <= root.level {
                entries.push_front(node);
                return Some(root);
            }
        }

        Some(root)
    }
}

pub struct HeadingRenderer {}

impl HeadingRenderer {
    fn new() -> Self {
        HeadingRenderer {}
    }
}

impl Default for HeadingRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadingAdapter for HeadingRenderer {
    fn enter(
        &self,
        output: &mut dyn std::io::Write,
        heading: &comrak::adapters::HeadingMeta,
        _sourcepos: Option<comrak::nodes::Sourcepos>,
    ) -> std::io::Result<()> {
        let slug = slugify(&heading.content);
        let tag = format!(
            "
          <a class=\"no-underline\" href=\"#{}\">
              <h{} id=\"{}\" class=\"group relative\">
              <span class=\"hidden group-hover:inline absolute -left-8\">#</span>
        ",
            slug, heading.level, slug
        );
        output.write_all(tag.as_bytes()).unwrap();
        Ok(())
    }

    fn exit(&self, output: &mut dyn std::io::Write, heading: &HeadingMeta) -> std::io::Result<()> {
        output
            .write_all(format!("</h{}></a>", heading.level).as_bytes())
            .unwrap();
        Ok(())
    }
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

    // TODO: reimplement this, now adding correct anchors to headings,
    // parsing images correctly and handling codeblocks better (maybe use
    // treesitter for it)
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
    pub author: String,

    #[serde(default)]
    pub base_url: String,

    #[serde(default = "default_true")]
    pub enable_rss: bool,

    #[serde(default = "default_empty")]
    pub extra_links_start: Vec<Link>,

    #[serde(default = "default_empty")]
    pub extra_links_end: Vec<Link>,

    // Used for adding live reload support in the templates
    #[serde(default)]
    pub is_dev_server: bool,
}

impl BlogConfiguration {
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        let config = toml::from_str(content)?;
        Ok(config)
    }
}

#[derive(Debug, Deserialize)]
pub struct BlogPostMetadata {
    pub title: String,
    pub authors: Option<Vec<String>>,
    pub slug: Option<String>,
    pub description: Option<String>,

    #[serde(with = "date")]
    pub date: NaiveDate,

    #[serde(default = "default_empty")]
    pub tags: Vec<String>,

    #[serde(default)]
    pub table_of_contents: bool,
}

impl BlogPostMetadata {
    pub fn format_date(&self) -> String {
        let date = self.date;
        let (_is_common_era, year) = date.year_ce();

        format!("{} {}, {}", format_month(date.month()), date.day(), year)
    }
}

fn format_month(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        n => panic!("unknown month: {}", n)
    }
}

#[derive(Debug)]
pub struct BlogPost<'a> {
    pub metadata: BlogPostMetadata,
    pub raw_content: String,
    pub ast: &'a AstNode<'a>,
    pub toc: TableOfContents,
    pub reading_time: u16,
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
    pub output_filename: PathBuf,
    pub filepath: &'a Path,
    pub description: String,
    pub rendered_html: String,
    pub images: Vec<PostImage>,
}

pub fn prepare_for_publish<'a>(
    p: &'a BlogPost<'a>,
    filepath: &'a Path,
    compiler: &'a PostCompiler<'a>,
) -> PublishableBlogPost<'a> {
    let images = map_images(p.ast);

    // ^ Operations that mutate AST nodes should be done before converting to HTML
    let rendered_html = compiler.ast_to_html(p.ast);

    let metadata = &p.metadata;
    let filename = match metadata.slug {
        Some(ref s) => slugify(s),
        None => slugify(&metadata.title),
    };
    let filename = Path::new(&filename).with_extension("html");

    let description = match p.metadata.description {
        Some(ref d) => d.to_owned(),
        None => build_description(p.ast),
    };

    PublishableBlogPost {
        post: p,
        filepath,
        output_filename: filename,
        description,
        rendered_html,
        images,
    }
}

fn build_description<'a>(ast: &'a AstNode<'a>) -> String {
    use comrak::nodes::NodeValue::*;

    for node in ast.traverse() {
        match node {
            comrak::arena_tree::NodeEdge::Start(nv) => {
                if let Paragraph = nv.data.borrow().value {
                    let mut buffer = String::new();

                    for c in nv.children() {
                        if let Text(ref t) = c.data.borrow().value {
                            buffer.push_str(t);
                            buffer.push(' ');
                        }
                    }

                    let description = buffer.split(' ').take(25).collect::<Vec<&str>>().join(" ");

                    return format!("{}...", description);
                }
            }
            comrak::arena_tree::NodeEdge::End(_nv) => continue,
        }
    }

    "".to_owned()
}

fn reading_time<'a>(ast: &'a AstNode<'a>) -> u16 {
    use comrak::nodes::NodeValue::*;
    let avg_words_per_minute = 225.0;
    let mut words_count = 0;

    for node in ast.traverse() {
        match node {
            comrak::arena_tree::NodeEdge::Start(nv) => match nv.data.borrow().value {
                Text(ref t) => {
                    words_count += wordcount(t).unwrap();
                }
                CodeBlock(ref b) => {
                    words_count += wordcount(&b.literal).unwrap();
                }
                _ => continue,
            },
            _ => continue,
        }
    }

    let average = (words_count as f64) / avg_words_per_minute;
    average.ceil() as u16
}

#[derive(Debug)]
pub struct PostImage {
    /// The path where an image can be found, relative to the blog's root
    pub original_path: String,

    /// The final path where the processed image will be found in the blog
    /// (e.g: /img/my-tour.png)
    pub final_path: PathBuf,
}

// TODO: Support image resizing and optimization (webp, responsive images)

// Walks the markdown AST and maps the images referenced in a post to the path
// they should have when publishing the blog
// This mutates the image nodes in the AST, changing their URL to their final
// path in the dist directory
fn map_images<'a>(ast: &'a AstNode<'a>) -> Vec<PostImage> {
    use comrak::nodes::NodeValue::*;

    // TODO: deduplicate images with the same name

    let mut post_images = Vec::new();

    for node in ast.borrow().traverse() {
        match node {
            comrak::arena_tree::NodeEdge::Start(nv) => match nv.data.borrow_mut().value {
                Image(ref mut i) => {
                    let path = Path::new(&i.url);
                    let filename = path.file_name().unwrap();
                    let final_path: PathBuf = filename.to_owned().into();

                    post_images.push(PostImage {
                        original_path: i.url.to_owned(),
                        final_path: final_path.to_owned(),
                    });

                    i.url = Path::new("/")
                        .join("img")
                        .join(filename)
                        .into_os_string()
                        .into_string()
                        .unwrap();
                }
                _ => continue,
            },
            _ => continue,
        }
    }

    post_images
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
            });
        }
    };

    let toc = TableOfContents::from_ast(ast);
    let reading_time = reading_time(ast);

    Ok(BlogPost {
        ast, // TODO: figure out how to have this mutable AST reference
        raw_content: content.to_owned(),
        reading_time,
        toc,
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
        }
        None => Err("could not find frontmatter section in file".to_owned()),
    }
}
