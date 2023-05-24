use std::{
    fs,
    path::{Path, PathBuf}
};

use comrak::{
    plugins::syntect::SyntectAdapter, Arena, ComrakExtensionOptions, ComrakOptions, ComrakPlugins,
};
use pageturtle_core::{
    build_blog_post, prepare_for_publish, BlogPost, PostCompiler, PublishableBlogPost, render_index,
};
use walkdir::WalkDir;

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

fn main() {
    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();

    let adapter = SyntectAdapter::new("base16-ocean.dark");
    let mut plugins = ComrakPlugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&adapter);
    let options = &ComrakOptions {
        extension: ComrakExtensionOptions {
            front_matter_delimiter: Some("---".to_owned()),
            ..ComrakExtensionOptions::default()
        },
        ..ComrakOptions::default()
    };

    let compiler = PostCompiler::new(arena, options, &plugins);

    let allowed_filetypes = vec!["md", "markdown"];
    let mut posts: Vec<BlogPost> = vec![];
    let mut failures: Vec<BuildPostError> = vec![];
    let blog_path = "pageturtle_cli/blog_template/posts";
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

        let content = fs::read_to_string(filepath).unwrap();

        match build_blog_post(&content, &compiler) {
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

    let mut publishable_posts: Vec<PublishableBlogPost> = posts
        .iter()
        .map(|p| prepare_for_publish(p, &compiler))
        .collect();

    publishable_posts.sort_by(|a, b| b.post.metadata.date.cmp(&a.post.metadata.date));

    // let arc_posts = Rc::new(publishable_posts);

    // create_search_index(&publishable_posts).unwrap(); // TODO: handle (create error type for file
                                                      // write error, etc...)

    // create_index_page(output_dir, &publishable_posts);


    // create index page
    // let posts_iter = &boxed_posts.iter();
    let index_html = render_index(&publishable_posts);
    let path = output_dir.join("index.html");
    fs::write(path, index_html).unwrap();

    for post in &publishable_posts {
        let path = output_dir.join(&post.filename);
        println!("writing file {:?}", path);
        fs::write(path, &post.rendered_html).unwrap();
    };

    fs::write(output_dir.join("styles.css"), pageturtle_core::stylesheet()).unwrap();

    dbg!(failures);
}

fn create_search_index<'a, I>(_posts: &I) -> Result<(), String>
where
    I: Iterator<Item = PublishableBlogPost<'a>>,
{
    Ok(())
}

fn create_posts<'a, I>(_output_dir: &Path, _posts: &'a I) -> Result<(), String>
where
    I: Iterator<Item = PublishableBlogPost<'a>>,
{
    Ok(())
}

fn create_index_page<'a, I>(_output_dir: &Path, _posts: &'a I)
where
    I: Iterator<Item = PublishableBlogPost<'a>>,
{
}
