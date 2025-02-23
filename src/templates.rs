use askama::Template;

use crate::ui::{LogGettItemView, LogListItemView, ProjectItemView};
use crate::ProjectItem;

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {}

#[derive(Template)]
#[template(path = "data.html")]
pub struct DataTemplate {
    pub projects: Vec<ProjectItemView>,
}

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

#[derive(Template)]
#[template(path = "projects.html")]
pub struct ProjectsTemplate<'a> {
    pub projects: &'a [ProjectItem],
}

#[derive(Template)]
#[template(path = "project_edit.html")]
pub struct ProjectEditTemplate {
    pub project: ProjectItemView,
}
