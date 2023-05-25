use std::{
    fs,
    path::{Path, PathBuf}, time::Instant, println,
};

use clap::{Subcommand, Parser};
use comrak::{
    plugins::syntect::SyntectAdapter, Arena, ComrakExtensionOptions, ComrakOptions, ComrakPlugins,
};
use pageturtle_core::{blog::{BlogPost, PostCompiler, build_blog_post, PublishableBlogPost, prepare_for_publish, BlogConfiguration}, rendering, self, feed};
use rouille::Response;
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
    Dev {
        #[clap(short, long, default_value_t = String::from("."), forbid_empty_values = true)]
        /// Blog directory
        directory: String,

        #[clap(short, long, default_value_t = 7000, forbid_empty_values = true)]
        /// Port that the development server will listen
        port: u32,

        #[clap(short, long, default_value_t = String::from("./dist"), forbid_empty_values = true)]
        /// Output directory
        output_directory: String,
    },
    /// Builds the blog
    Build {
        #[clap(short, long, default_value_t = String::from("."), forbid_empty_values = true)]
        /// Blog directory
        directory: String,

        #[clap(short, long, default_value_t = String::from("./dist"), forbid_empty_values = true)]
        /// Output directory
        output_directory: String,
    },
    /// Stars a new blog
    Init {
        #[clap(short, long, default_value_t = String::from("."), forbid_empty_values = true)]
        /// Blog directory
        directory: String,
    },
}

fn main() {
    let cli = CLI::parse();
    match &cli.command {
        Command::Build { directory, output_directory } => {
            let blog_root = Path::new(directory);
            let config = read_config(blog_root);

            let start = Instant::now();
            build(blog_root, Path::new(output_directory), &config);

            let duration = start.elapsed();
            println!("Succesfully build blog in {:?}", duration);
        }
        Command::Init { directory } => {
            let path = Path::new(directory);
            match init_blog(path) {
                Ok(()) => {
                    println!("Blog succesfully started at {}", path.display())
                }
                Err(msg) => 
                    println!("Failed to init blog at {:?}: {}", path.display(), msg)
            }
        },
        Command::Dev { port, directory, output_directory } => {
            start_dev_server(*port, Path::new(directory), Path::new(output_directory));
        },
    }
}

fn build(blog_root: &Path, output_directory: &Path, config: &BlogConfiguration) {
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
    let posts_dir = blog_root.join("posts");

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

fn init_blog(target_directory: &Path) -> Result<(), String> {
    let config = include_bytes!("other/config.toml");
    let getting_started = include_bytes!("other/getting_started.md");

    if target_directory.exists() && !target_directory.is_dir() {
        return Err("output path already exists and is not a directory".to_owned())
    }

    if target_directory.exists() 
        && target_directory.is_dir() 
        && target_directory.read_dir().unwrap().next().is_some() {
        return Err("output directory already exists and is not empty".to_owned());
    }

    if !target_directory.exists() {
        fs::create_dir_all(target_directory).unwrap();
    }

    fs::write(target_directory.join("config.toml"), config).unwrap();

    let posts_dir = target_directory.join("posts");
    fs::create_dir_all(&posts_dir).unwrap();
    fs::write(posts_dir.join("getting_started.md"), getting_started).unwrap();

    Ok(())
}

fn write_blog() {

}

fn start_dev_server(port: u32, blog_root: &Path, output_directory: &Path) {
    let host = format!("localhost:{}", port);
    let output = output_directory.to_owned();

    println!("pageturtle server listening on {}", host);

    let config = BlogConfiguration {
        base_url: format!("http://{}", host),
        ..read_config(blog_root)
    };

    // TODO: watch FS and implement websocket for live reload
    build(&blog_root, &output_directory, &config);

    rouille::start_server(host, move |request| {
    {
        if request.url() == "/" {
            return Response::redirect_303("/index.html");
        }

        // The `match_assets` function tries to find a file whose name corresponds to the URL
        // of the request. The second parameter (`"."`) tells where the files to look for are
        // located.
        // In order to avoid potential security threats, `match_assets` will never return any
        // file outside of this directory even if the URL is for example `/../../foo.txt`.
        let response = rouille::match_assets(&request, &output);

        // If a file is found, the `match_assets` function will return a response with a 200
        // status code and the content of the file. If no file is found, it will instead return
        // an empty 404 response.
        // Here we check whether if a file is found, and if so we return the response.
        if response.is_success() {
            return response;
        }
    }

    // This point of the code is reached only if no static file matched the request URL.

    // In a real website you probably want to serve non-static files here (with the `router!`
    // macro for example), but here we just return a 404 response.
    Response::html("404 file not found error").with_status_code(404)
});
}

fn read_config(blog_root: &Path) -> BlogConfiguration {
    let config_file = fs::read_to_string(blog_root.join("config.toml")).unwrap();
    BlogConfiguration::from_toml(&config_file).unwrap()
}
