use once_cell::sync::Lazy;
use quick_xml::{events::BytesText, writer::Writer};

pub static CAPABILITIES: Lazy<String> = Lazy::new(build_capabilities);

fn build_capabilities() -> String {
    tracing::debug!("building capabilities");
    let mut writer = quick_xml::writer::Writer::new(Vec::new());
    write_caps(&mut writer).expect("build capabilities xml");
    let inner = writer.into_inner();
    let result = String::from_utf8_lossy(&inner);
    format!("{}{result}", super::DOM)
}

fn write_caps(writer: &mut Writer<Vec<u8>>) -> quick_xml::Result<()> {
    writer.create_element("caps").write_inner_content(|w| {
        write_server(w)?;
        write_limits(w)?;
        write_searching(w)?;
        write_categories(w)?;
        Ok(())
    })?;
    Ok(())
}

fn write_server(writer: &mut Writer<Vec<u8>>) -> quick_xml::Result<()> {
    writer
        .create_element("server")
        .write_text_content(BytesText::new(env!("CARGO_PKG_NAME")))?;
    Ok(())
}

fn write_limits(writer: &mut Writer<Vec<u8>>) -> quick_xml::Result<()> {
    writer
        .create_element("limits")
        .with_attribute(("default", "100"))
        .with_attribute(("max", "100"))
        .write_empty()?;
    Ok(())
}

fn write_searching(writer: &mut Writer<Vec<u8>>) -> quick_xml::Result<()> {
    writer
        .create_element("searching")
        .write_inner_content(|w| {
            w.create_element("search")
                .with_attribute(("available", "yes"))
                .with_attribute(("supportedParams", "q"))
                .write_empty()?;
            w.create_element("tv-search")
                .with_attribute(("available", "yes"))
                .with_attribute(("supportedParams", "q,season,ep"))
                .write_empty()?;
            w.create_element("movie-search")
                .with_attribute(("available", "yes"))
                .with_attribute(("supportedParams", "q"))
                .write_empty()?;
            w.create_element("music-search")
                .with_attribute(("available", "yes"))
                .with_attribute(("supportedParams", "q"))
                .write_empty()?;
            w.create_element("book-search")
                .with_attribute(("available", "yes"))
                .with_attribute(("supportedParams", "q"))
                .write_empty()?;
            Ok(())
        })?;
    Ok(())
}

fn write_categories(writer: &mut Writer<Vec<u8>>) -> quick_xml::Result<()> {
    writer
        .create_element("categories")
        .write_inner_content(|w| {
            w.create_element("category")
                .with_attribute(("id", "2000"))
                .with_attribute(("name", "Movies"))
                .write_empty()?;
            w.create_element("category")
                .with_attribute(("id", "3000"))
                .with_attribute(("name", "Audio"))
                .write_empty()?;
            w.create_element("category")
                .with_attribute(("id", "5000"))
                .with_attribute(("name", "TV"))
                .write_empty()?;
            w.create_element("category")
                .with_attribute(("id", "7000"))
                .with_attribute(("name", "Books"))
                .write_empty()?;
            Ok(())
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn expected_result() {
        assert_eq!(
            super::build_capabilities(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><caps><server>manteau</server><limits default=\"100\" max=\"100\"/><searching><search available=\"yes\" supportedParams=\"q\"/><tv-search available=\"yes\" supportedParams=\"q,season,ep\"/><movie-search available=\"yes\" supportedParams=\"q\"/><music-search available=\"yes\" supportedParams=\"q\"/><book-search available=\"yes\" supportedParams=\"q\"/></searching><categories><category id=\"2000\" name=\"Movies\"/><category id=\"3000\" name=\"Audio\"/><category id=\"5000\" name=\"TV\"/><category id=\"7000\" name=\"Books\"/></categories></caps>"
        );
    }
}
