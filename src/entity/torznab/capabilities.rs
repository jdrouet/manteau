use once_cell::sync::Lazy;

pub static CAPABILITIES: Lazy<String> = Lazy::new(|| {
    tracing::debug!("building capabilities");
    let mut builder = String::from(super::DOM);
    Capabilities::default().push_xml(&mut builder);
    builder
});

#[derive(Debug, Default, PartialEq)]
pub struct Capabilities {
    server: Server,
    limits: Limits,
    searching: Searching,
    categories: Categories,
}

impl Capabilities {
    pub fn push_xml(&self, builder: &mut String) {
        builder.push_str("<caps>");
        self.server.push_xml(builder);
        self.limits.push_xml(builder);
        self.searching.push_xml(builder);
        self.categories.push_xml(builder);
        builder.push_str("</caps>");
    }
}

#[derive(Debug, PartialEq)]
pub struct Server {
    title: &'static str,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            title: env!("CARGO_PKG_NAME"),
        }
    }
}

impl Server {
    pub fn push_xml(&self, builder: &mut String) {
        builder.push_str("<server>");
        builder.push_str(self.title);
        builder.push_str("</server>");
    }
}

#[derive(Debug, PartialEq)]
pub struct Limits {
    default: u32,
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

impl Limits {
    pub fn push_xml(&self, builder: &mut String) {
        builder.push_str(&format!(
            "<limits default=\"{}\" max=\"{}\" />",
            self.default, self.max
        ));
    }
}

#[derive(Debug, PartialEq)]
pub struct Searching([Search; 5]);

impl Default for Searching {
    fn default() -> Self {
        Self([
            Search::new("search", "yes", "q"),
            Search::new("tv-search", "yes", "q,season,ep"),
            Search::new("movie-search", "yes", "q"),
            Search::new("music-search", "yes", "q"),
            Search::new("book-search", "yes", "q"),
        ])
    }
}

impl Searching {
    pub fn push_xml(&self, builder: &mut String) {
        builder.push_str("<searching>");
        self.0.iter().for_each(|s| s.push_xml(builder));
        builder.push_str("</searching>");
    }
}

#[derive(Debug, PartialEq)]
pub struct Search {
    tag: &'static str,
    available: &'static str,
    supported_params: &'static str,
}

impl Search {
    pub const fn new(
        tag: &'static str,
        available: &'static str,
        supported_params: &'static str,
    ) -> Self {
        Self {
            tag,
            available,
            supported_params,
        }
    }

    pub fn push_xml(&self, builder: &mut String) {
        builder.push_str(&format!(
            r#"<{} available={:?} supportedParams={:?} />"#,
            self.tag, self.available, self.supported_params
        ));
    }
}

#[derive(Debug, PartialEq, serde::Serialize)]
#[serde(rename = "categories")]
pub struct Categories([Category; 4]);

impl Default for Categories {
    fn default() -> Self {
        Self([
            Category::new(2000, "Movies"),
            Category::new(3000, "Audio"),
            Category::new(5000, "TV"),
            Category::new(7000, "Books"),
        ])
    }
}

impl Categories {
    pub fn push_xml(&self, builder: &mut String) {
        builder.push_str("<categories>");
        self.0.iter().for_each(|c| c.push_xml(builder));
        builder.push_str("</categories>");
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

    pub fn push_xml(&self, builder: &mut String) {
        builder.push_str(&format!(
            "<category id=\"{}\" name={:?} />",
            self.id, self.name
        ));
    }
}
