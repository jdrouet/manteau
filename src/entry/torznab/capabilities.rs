use once_cell::sync::Lazy;

pub static CAPABILITIES: Lazy<String> = Lazy::new(|| {
    tracing::debug!("building capabilities");
    let caps = Capabilities::default();
    let body = quick_xml::se::to_string(&caps).unwrap();
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>{body}")
});

#[derive(Debug, Default, PartialEq, serde::Serialize)]
#[serde(rename = "caps")]
pub struct Capabilities {
    server: Server,
    limits: Limits,
    searching: Searching,
    categories: Categories,
}

#[derive(Debug, PartialEq, serde::Serialize)]
#[serde(rename = "server")]
pub struct Server {
    #[serde(rename = "@title")]
    title: &'static str,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            title: env!("CARGO_PKG_NAME"),
        }
    }
}

#[derive(Debug, PartialEq, serde::Serialize)]
#[serde(rename = "limits")]
pub struct Limits {
    #[serde(rename = "@default")]
    default: u32,
    #[serde(rename = "@max")]
    max: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            default: 100,
            max: 100,
        }
    }
}

#[derive(Debug, PartialEq, serde::Serialize)]
#[serde(rename = "searching")]
pub struct Searching {
    #[serde(rename = "search")]
    search: Search,
    #[serde(rename = "tv-search")]
    tv_search: Search,
    #[serde(rename = "movie-search")]
    movie_search: Search,
    #[serde(rename = "music-search")]
    music_search: Search,
    #[serde(rename = "book-search")]
    book_search: Search,
}

impl Default for Searching {
    fn default() -> Self {
        Self {
            search: Search {
                available: "yes",
                supported_params: "q",
            },
            tv_search: Search {
                available: "yes",
                supported_params: "q,season,ep",
            },
            movie_search: Search {
                available: "yes",
                supported_params: "q",
            },
            music_search: Search {
                available: "yes",
                supported_params: "q",
            },
            book_search: Search {
                available: "yes",
                supported_params: "q",
            },
        }
    }
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct Search {
    #[serde(rename = "@available")]
    available: &'static str,
    #[serde(rename = "@supported-params")]
    supported_params: &'static str,
}

#[derive(Debug, PartialEq, serde::Serialize)]
#[serde(rename = "categories")]
pub struct Categories {
    inner: [Category; 4],
}

impl Default for Categories {
    fn default() -> Self {
        Self {
            inner: [
                Category::new(2000, "Movies"),
                Category::new(3000, "Audio"),
                Category::new(5000, "TV"),
                Category::new(7000, "Books"),
            ],
        }
    }
}

#[derive(Debug, PartialEq, serde::Serialize)]
#[serde(rename = "category")]
pub struct Category {
    #[serde(rename = "@id")]
    id: u32,
    #[serde(rename = "@name")]
    name: &'static str,
}

impl Category {
    pub const fn new(id: u32, name: &'static str) -> Self {
        Self { id, name }
    }
}
