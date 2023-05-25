use std::{
    fs,
    path::{Path, PathBuf}, time::Instant,
};

use clap::{Subcommand, Parser};
use comrak::{
    plugins::syntect::SyntectAdapter, Arena, ComrakExtensionOptions, ComrakOptions, ComrakPlugins,
};
use pageturtle_core::{blog::{BlogPost, PostCompiler, build_blog_post, PublishableBlogPost, prepare_for_publish, BlogConfiguration}, rendering, self, feed};
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

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct CLI {
    #[clap(subcommand)]
    command: Command,
}


#[derive(Debug, Subcommand)]
enum Command {
    /// Count how many times the package is used
    Watch {
        #[clap(short, long, default_value_t = String::from("."), forbid_empty_values = true)]
        /// Blog directory
        blog_directory: String,

        #[clap(short, long, default_value_t = String::from("./dist"), forbid_empty_values = true)]
        /// Output directory
        output_directory: String,
    },
    /// Builds the project
    Build {
        #[clap(short, long, default_value_t = String::from("."), forbid_empty_values = true)]
        /// Blog directory
        blog_directory: String,

        #[clap(short, long, default_value_t = String::from("./dist"), forbid_empty_values = true)]
        /// Output directory
        output_directory: String,
    },
}

fn main() {
    let cli = CLI::parse();
    match &cli.command {
        Command::Build { blog_directory, output_directory } => {
            let start = Instant::now();
            build(&blog_directory, &output_directory);
            let duration = start.elapsed();
            println!("Succesfully build blog in {:?}", duration);
        }
        Command::Watch { blog_directory: _, output_directory: _ } => {
            todo!()
        },
    }
}

fn build(blog_directory: &str, output_directory: &str) {
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
    let blog_path = Path::new(blog_directory);
    let posts_dir = blog_path.join("posts");

    let config_file = fs::read_to_string(blog_path.join("config.toml")).unwrap();
    let config = BlogConfiguration::from_toml(&config_file).unwrap();

    // TODO: parse files in parallel
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

    let output_dir = Path::new(output_directory);

    if !output_dir.exists() {
        fs::create_dir_all(output_dir).unwrap();
    }

    let mut publishable_posts: Vec<PublishableBlogPost> = posts
        .iter()
        .map(|p| prepare_for_publish(p, &compiler))
        .collect();

    publishable_posts.sort_by(|a, b| b.post.metadata.date.cmp(&a.post.metadata.date));

    // create index page
    let index_html = rendering::render_index(&publishable_posts, &config);
    let path = output_dir.join("index.html");
    fs::write(path, index_html).unwrap();

    // create tags page
    let tags_html = rendering::render_tags_page(&posts, &config);
    let tags_path = output_dir.join("tags.html");
    fs::write(tags_path, tags_html).unwrap();

    // write posts
    for post in &publishable_posts {
        let path = output_dir.join(&post.filename);
        // println!("writing file {:?}", path);
        let page = rendering::render_post_page(post, &config);
        fs::write(path, page).unwrap();
    }

    // write rss feed
    if config.enable_rss {
        let feed = feed::build_feed(&publishable_posts, &config);
        let feed_xml = rendering::render_feed(&feed);
        fs::write(output_dir.join("atom.xml"), feed_xml).unwrap();
    }

    fs::write(output_dir.join("styles.css"), rendering::stylesheet()).unwrap();
}
