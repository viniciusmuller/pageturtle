use std::{
    fs,
    path::{Path, PathBuf},
};

use comrak::{
    plugins::syntect::SyntectAdapter, Arena, ComrakExtensionOptions, ComrakOptions, ComrakPlugins,
};
use pageturtle_core::{
    build_blog_post, parse_config, prepare_for_publish, render_index, render_post_page,
    render_tags_page, BlogPost, PostCompiler, PublishableBlogPost,
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
    let blog_path = Path::new("pageturtle_cli/blog_template");
    let posts_dir = blog_path.join("posts");
    let output_target = "dist";

    let config_file = fs::read_to_string(blog_path.join("config.toml")).unwrap();
    let config = parse_config(&config_file).unwrap();

    let walker = WalkDir::new(posts_dir).into_iter();
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

    // create index page
    let index_html = render_index(&publishable_posts, &config);
    let path = output_dir.join("index.html");
    fs::write(path, index_html).unwrap();

    let tags_html = render_tags_page(&posts, &config);
    let tags_path = output_dir.join("tags.html");
    fs::write(tags_path, tags_html).unwrap();

    for post in &publishable_posts {
        let path = output_dir.join(&post.filename);
        println!("writing file {:?}", path);
        let page = render_post_page(post, &config);
        fs::write(path, page).unwrap();
    }

    fs::write(output_dir.join("styles.css"), pageturtle_core::stylesheet()).unwrap();

    dbg!(failures);
}
