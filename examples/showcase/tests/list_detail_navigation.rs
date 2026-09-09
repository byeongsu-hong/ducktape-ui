mod projects {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Project {
        pub id: i64,
        pub title: String,
        pub description: String,
        pub metadata: String,
        pub initials: String,
        pub draft: String,
    }

    pub fn initial() -> Vec<Project> {
        [
            (10, "Design system", "Shared controls and documentation", "Ready", "DS"),
            (20, "Quarterly workspace migration and accessibility review", "Coordinate the platform rollout with every workspace owner before the final review.", "Updated yesterday by Robin", "QR"),
            (30, "Research", "", "", "R"),
        ]
        .into_iter()
        .map(|(id, title, description, metadata, initials)| Project {
            id,
            title: title.into(),
            description: description.into(),
            metadata: metadata.into(),
            initials: initials.into(),
            draft: format!("Notes for {title}"),
        })
        .collect()
    }

    pub fn visible(projects: &[Project], query: &str, reversed: bool) -> Vec<Project> {
        let query = query.trim().to_lowercase();
        let mut rows: Vec<_> = projects
            .iter()
            .filter(|project| project.title.to_lowercase().contains(&query))
            .cloned()
            .collect();
        if reversed {
            rows.reverse();
        }
        rows
    }

    pub fn selected(projects: &[Project], id: i64) -> Project {
        projects
            .iter()
            .find(|project| project.id == id)
            .expect("known project ID")
            .clone()
    }

    pub fn edit(mut projects: Vec<Project>, id: i64, draft: String) -> Vec<Project> {
        projects
            .iter_mut()
            .find(|project| project.id == id)
            .expect("known project ID")
            .draft = draft;
        projects
    }
}

ui_lang::include_app!("tests/cases/ui/list_detail_navigation.ice");
