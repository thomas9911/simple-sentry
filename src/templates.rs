use askama::Template;

use crate::ui::LogListItemView;

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
