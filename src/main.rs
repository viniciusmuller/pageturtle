use serde::Deserialize;
use std::{todo, borrow::Borrow, cell::RefCell};

use comrak::{nodes::Ast, Arena, ComrakExtensionOptions, ComrakOptions, arena_tree::Node};

#[derive(Debug, Deserialize)]
struct BlogPostSettings {
    title: String,
    date: String,
    tags: Option<Vec<String>>,
}

fn main() {
    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();

    let md = std::fs::read_to_string("fable_react_interop_guide.md").unwrap();

    let options = &ComrakOptions {
        extension: ComrakExtensionOptions {
            front_matter_delimiter: Some("---".to_owned()),
            ..ComrakExtensionOptions::default()
        },
        ..ComrakOptions::default()
    };

    let ast = comrak::parse_document(&arena, &md, options);

    let settings = match parse_frontmatter(ast) {
        Ok(settings) => settings,
        Err(msg) => panic!("{:#?}", msg) // TODO: figure out how exit gracefully
    };

    dbg!("{:#?}", settings);

    let mut vec = Vec::new();

    match comrak::format_html(ast, options, &mut vec) {
        Ok(_) => {
            let _result_html = String::from_utf8(vec).unwrap();
        }
        Err(_) => todo!(),
    }
}

fn parse_frontmatter<'a>(ast: &'a Node<'a, RefCell<Ast>>) -> Result<BlogPostSettings, String> {
    use comrak::nodes::NodeValue::*;

    let mut frontmatter: Option<String> = None;

    for node in ast.borrow().traverse() {
        match node {
            comrak::arena_tree::NodeEdge::Start(nv) => {
                if let FrontMatter(s) = &nv.borrow().data.borrow().value {
                    frontmatter = Some(s.to_owned())
                }
            }
            comrak::arena_tree::NodeEdge::End(_nv) => {
                break
            },
        }
    };

    match frontmatter {
        Some(s) => parse_frontmatter_yaml(&s),
        None => panic!("could not find frontmatter in file: ...")
    }
}

fn parse_frontmatter_yaml(s: &str) -> Result<BlogPostSettings, String> {
    let unprefixed =  unquote_frontmatter(s);
    match serde_yaml::from_str::<BlogPostSettings>(&unprefixed) {
        Ok(settings) => Ok(settings),
        Err(e) => Err(e.to_string())
    }
}

fn unquote_frontmatter(fm: &str) -> String {
    fm.replace("---", "")
}
