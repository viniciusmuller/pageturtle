use chrono::{DateTime, Utc};
use serde::Deserialize;
use slug::slugify;
use std::{
    borrow::Borrow,
    cell::RefCell,
    dbg, fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

use comrak::{arena_tree::Node, nodes::Ast, Arena, ComrakExtensionOptions, ComrakOptions, ComrakPlugins};
use comrak::plugins::syntect::SyntectAdapter;

#[derive(Debug, Deserialize)]
struct BlogPostMetadata {
    title: String,
    slug: Option<String>,
    #[serde(with = "date")]
    date: DateTime<Utc>,
    tags: Option<Vec<String>>,
}

mod date {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer};

    const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%SZ";

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
struct BlogPost {
    metadata: BlogPostMetadata,
    raw_content: String,
    rendered_html: String,
}

#[derive(Debug)]
/// Error that can happen when building a post from a filepath.
/// Contains OS-level metadata such as filepath or file content.
struct BuildPostError {
    filepath: PathBuf,
    content: String,
    line: u32,
    column: u32,
    message: String,
}

#[derive(Debug)]
/// Error that can happen when parsing a post and compiling it to HTML
struct CompilePostError {
    line: u32,
    column: u32,
    message: String,
}

fn main() {
    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();

    let adapter = SyntectAdapter::new("base16-ocean.dark");
    let mut plugins = ComrakPlugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&adapter);

    let allowed_filetypes = vec!["md", "markdown"];
    let mut posts: Vec<BlogPost> = vec![];
    let mut failures: Vec<BuildPostError> = vec![];
    let blog_path = "template/posts";
    let output_target = "dist";

    let walker = WalkDir::new(blog_path).min_depth(1).into_iter();
    for entry in walker {
        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            continue;
        };

        let filepath = entry.path();
        match filepath.extension() {
            Some(e) => {
                if !allowed_filetypes.contains(&e.to_str().unwrap()) {
                    continue;
                }
            }
            None => continue,
        }

        let content = fs::read_to_string(&filepath).unwrap();

        match build_blog_post(&arena, &content, &plugins) {
            Ok(post) => posts.push(post),
            Err(e) => failures.push(BuildPostError {
                filepath: filepath.into(),
                content,
                line: e.line,
                column: e.column,
                message: e.message,
            }),
        };
    }

    let output_dir = Path::new(output_target);

    if !output_dir.exists() {
        fs::create_dir_all(output_dir).unwrap();
    }

    for post in posts {
        let html_document = render_post(&post);
        let filename = match post.metadata.slug {
            Some(s) => slugify(s),
            None => slugify(post.metadata.title),
        };
        let path = output_dir.join(filename).with_extension("html");
        fs::write(path, html_document).unwrap();
    }

    dbg!(failures);
}

fn build_blog_post<'a>(
    arena: &'a Arena<Node<'a, RefCell<Ast>>>,
    content: &str,
    plugins: &ComrakPlugins
) -> Result<BlogPost, CompilePostError> {
    let options = &ComrakOptions {
        extension: ComrakExtensionOptions {
            front_matter_delimiter: Some("---".to_owned()),
            ..ComrakExtensionOptions::default()
        },
        ..ComrakOptions::default()
    };

    let ast = comrak::parse_document(arena, content, options);

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

    let mut vec = Vec::new();

    match comrak::format_html_with_plugins(ast, options, &mut vec, plugins) {
        Ok(_) => {
            let result_html = String::from_utf8(vec).unwrap();
            Ok(BlogPost {
                raw_content: content.to_owned(),
                metadata,
                rendered_html: result_html,
            })
        }
        Err(e) => Err(CompilePostError {
            line: 0,
            column: 0,
            message: e.to_string(),
        }),
    }
}

// TODO: improve how templating works
const TEMPLATE: &'static str = "
<html>
  <title>
    {title}
  </title>

  <head>
    <script src='https://cdn.tailwindcss.com?plugins=forms,typography,aspect-ratio,line-clamp'></script>
  </head>

  <body class='flex justify-center'>
    <article class='prose mt-8'>
      <h1>
        {title}
      </h1>
      {content}
    </article>
  </body>
</html>
";

fn render_post(post: &BlogPost) -> String {
    TEMPLATE
        .replace("{title}", &post.metadata.title)
        .replace("{content}", &post.rendered_html)
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
    let unprefixed = unquote_frontmatter(s);
    match serde_yaml::from_str::<BlogPostMetadata>(&unprefixed) {
        Ok(settings) => Ok(settings),
        Err(e) => Err(e.to_string()),
    }
}

fn unquote_frontmatter(fm: &str) -> String {
    fm.replace("---", "")
}
