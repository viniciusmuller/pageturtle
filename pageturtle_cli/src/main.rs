use std::{
    fs,
    path::{Path, PathBuf},
    println, thread,
    time::Instant,
};

use clap::{Parser, Subcommand};
use comrak::{
    plugins::syntect::SyntectAdapter, Arena, ComrakExtensionOptions, ComrakOptions, ComrakPlugins,
};
use crossbeam_channel::{unbounded, Receiver};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use pageturtle_core::{
    self,
    blog::{
        build_blog_post, prepare_for_publish, BlogConfiguration, BlogPost, PostCompiler,
        PublishableBlogPost, HeadingRenderer,
    },
    feed, rendering,
};
use rouille::{router, try_or_400, websocket, Response};
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
struct Cli {
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

        #[clap(short, long, forbid_empty_values = true)]
        /// Output directory
        output_directory: Option<String>,
    },
    /// Builds the blog
    Build {
        #[clap(short, long, default_value_t = String::from("."), forbid_empty_values = true)]
        /// Blog directory
        directory: String,

        #[clap(short, long, forbid_empty_values = true)]
        /// Output directory
        output_directory: Option<String>,
    },
    /// Stars a new blog
    Init {
        #[clap(short, long, default_value_t = String::from("."), forbid_empty_values = true)]
        /// Blog directory
        directory: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Command::Build {
            directory,
            output_directory,
        } => {
            let blog_root = Path::new(directory);
            let output = match output_directory {
                Some(o) => Path::new(o).to_owned(),
                None => blog_root.join("dist"),
            };

            let config = read_config(blog_root);

            let start = Instant::now();
            build(blog_root, &output, &config);
            let duration = start.elapsed();
            println!("Succesfully build blog in {:?}", duration);
        }
        Command::Init { directory } => {
            let path = Path::new(directory);
            match init_blog(path) {
                Ok(()) => {
                    println!("Blog succesfully started at {}", path.display())
                }
                Err(msg) => println!("Failed to init blog at {:?}: {}", path.display(), msg),
            }
        }
        Command::Dev {
            port,
            directory,
            output_directory,
        } => {
            let root = Path::new(directory);
            let output = match output_directory {
                Some(o) => Path::new(o).to_owned(),
                None => root.join("dist"),
            };

            start_dev_server(*port, root, &output);
        }
    }
}

fn build(blog_root: &Path, output_directory: &Path, config: &BlogConfiguration) {
    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();

    // let adapter = SyntectAdapter::new("base16-ocean.dark");
    // plugins.render.codefence_syntax_highlighter = Some(&adapter);
    let options = &ComrakOptions {
        extension: ComrakExtensionOptions {
            front_matter_delimiter: Some("---".to_owned()),
            ..ComrakExtensionOptions::default()
        },
        ..ComrakOptions::default()
    };

    let adapter = HeadingRenderer::new();
    let mut plugins = ComrakPlugins::default();
    plugins.render.heading_adapter = Some(&adapter);

    let compiler = PostCompiler::new(arena, &options, &plugins);

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
                if !check_allowed_filetype(e.to_str().unwrap()) {
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
    let index_html = rendering::render_index(&publishable_posts, config);
    let path = output_dir.join("index.html");
    fs::write(path, index_html).unwrap();

    // create tags page
    let tags_html = rendering::render_tags_page(&posts, config);
    let tags_path = output_dir.join("tags.html");
    fs::write(tags_path, tags_html).unwrap();

    // write posts
    for post in &publishable_posts {
        let path = output_dir.join(&post.filename);
        // println!("writing file {:?}", path);
        let page = rendering::render_post_page(post, config);
        fs::write(path, page).unwrap();
    }

    // write rss feed
    if config.enable_rss {
        let feed = feed::build_feed(&publishable_posts, config);
        let feed_xml = rendering::render_feed(&feed);
        fs::write(output_dir.join("atom.xml"), feed_xml).unwrap();
    }

    dbg!(&failures);

    fs::write(output_dir.join("styles.css"), rendering::stylesheet()).unwrap();
}

fn init_blog(target_directory: &Path) -> Result<(), String> {
    let config = include_bytes!("other/config.toml");
    let getting_started = include_bytes!("other/getting_started.md");

    if target_directory.exists() && !target_directory.is_dir() {
        return Err("output path already exists and is not a directory".to_owned());
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

fn start_dev_server(port: u32, blog_root: &Path, output_directory: &Path) {
    let output_2 = output_directory.to_owned();
    let output = output_directory.to_owned();

    let host = format!("localhost:{}", port);
    let config = BlogConfiguration {
        base_url: format!("http://{}", host),
        is_dev_server: true,
        ..read_config(blog_root)
    };

    build(blog_root, output_directory, &config);

    // Create a channel to receive the events.
    let (event_tx, event_rx) = unbounded();
    let (changes_tx, changes_rx) = unbounded();

    let changes_rx_2 = changes_rx;

    let root = blog_root.to_owned();

    thread::spawn(move || {
        let mut watcher = RecommendedWatcher::new(event_tx, Config::default()).unwrap();

        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.
        watcher.watch(&root, RecursiveMode::Recursive).unwrap();

        for res in event_rx {
            match res {
                Ok(Event {
                    kind,
                    paths,
                    attrs: _,
                }) => {
                    match kind {
                        notify::EventKind::Create(_) => {}
                        notify::EventKind::Modify(_) | notify::EventKind::Remove(_) => {
                            // TODO: prevent duplicate entries when saving a file
                            // currently it is building more than once unnecessarily
                            let path = paths.first().unwrap();
                            if let Some(ext) = path.extension() {
                                if check_allowed_filetype(ext.to_str().unwrap()) {
                                    let start = Instant::now();
                                    build(&root, &output, &config);
                                    let duration = start.elapsed();
                                    println!("[rebuilt] {:?}", duration);
                                    changes_tx.send(path.clone()).unwrap();
                                }
                            }
                        }
                        _ => (),
                    }
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    });

    thread::spawn(move || {
        let host = format!("localhost:{}", port);

        println!("pageturtle server listening on {}", &host);
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
                let response = rouille::match_assets(request, &output_2);

                // If a file is found, the `match_assets` function will return a response with a 200
                // status code and the content of the file. If no file is found, it will instead return
                // an empty 404 response.
                // Here we check whether if a file is found, and if so we return the response.
                if response.is_success() {
                    return response;
                }
            }

            // This point of the code is reached only if no static file matched the request URL.

            router!(request,
                (GET) (/ws) => {
                    // This is the websockets route.

                    // In order to start using websockets we call `websocket::start`.
                    // The function returns an error if the client didn't request websockets, in which
                    // case we return an error 400 to the client thanks to the `try_or_400!` macro.
                    //
                    // The function returns a response to send back as part of the `start_server`
                    // function, and a `websocket` variable of type `Receiver<Websocket>`.
                    // Once the response has been sent back to the client, the `Receiver` will be
                    // filled by rouille with a `Websocket` object representing the websocket.
                    let (response, websocket) = try_or_400!(websocket::start(request, Some("handshake")));

                    // Because of the nature of I/O in Rust, we need to spawn a separate thread for
                    // each websocket.
                    let changes_rx_3 = changes_rx_2.clone();

                    thread::spawn(move || {
                        // This line will block until the `response` above has been returned.
                        let ws = websocket.recv().unwrap();
                        // We use a separate function for better readability.
                        // TODO: figure out about the probalby certain race conditon/batching that seems to be occuring
                        websocket_handling_thread(ws, changes_rx_3);
                    });

                    response
                },
                _ => Response::empty_404()
            )
        });
    }).join().unwrap();
}

// Function run in a separate thread.
fn websocket_handling_thread(mut websocket: websocket::Websocket, rx: Receiver<PathBuf>) {
    for msg in rx {
        match websocket.send_text(msg.to_str().unwrap()) {
            Ok(_) => (),
            Err(_) => return, // probably the WS was closed
        };
    }
}

fn check_allowed_filetype(extension: &str) -> bool {
    vec!["md", "markdown"].contains(&extension)
}

fn read_config(blog_root: &Path) -> BlogConfiguration {
    let config_file = fs::read_to_string(blog_root.join("config.toml")).unwrap();
    BlogConfiguration::from_toml(&config_file).unwrap()
}
