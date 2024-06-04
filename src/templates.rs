use askama::Template;

use crate::ui::{LogGettItemView, LogListItemView};

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {}

#[derive(Template)]
#[template(path = "data.html")]
pub struct DataTemplate {}

#[derive(Template)]
#[template(path = "data_contents.html")]
pub struct DataContentsTemplate {
    pub entries: Vec<LogListItemView>,
}

#[derive(Template)]
#[template(path = "data_content.html")]
pub struct DataContentTemplate {
    pub entry: LogGettItemView,
}
