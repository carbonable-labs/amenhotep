use serde::Serialize;
use thiserror::Error;

use crate::parser::{CairoEvent, FileDomain};

#[derive(Serialize)]
pub(crate) struct CheckpointConfiguration {
    pub network_node_url: String,
    pub sources: Vec<CheckpointSource>,
}

impl From<&[FileDomain]> for CheckpointConfiguration {
    fn from(value: &[FileDomain]) -> Self {
        let sources = value
            .iter()
            .filter(|fd| !fd.events.is_empty())
            .map(|fd| fd.into())
            .collect::<Vec<CheckpointSource>>();
        Self {
            network_node_url: "<CHANGE_ME>".to_string(),
            sources,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct CheckpointSource {
    pub contract: String,
    pub start: u64,
    pub deploy_fn: String,
    pub events: Vec<CheckpointEvent>,
}

impl From<&FileDomain> for CheckpointSource {
    fn from(value: &FileDomain) -> Self {
        let events = value
            .events
            .iter()
            .map(|e| e.into())
            .collect::<Vec<CheckpointEvent>>();
        Self {
            contract: "<CHANGE_ME>".to_string(),
            start: 0,
            deploy_fn: "handleDeploy".to_string(),
            events,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct CheckpointEvent {
    pub name: String,
    #[serde(rename = "fn")]
    pub function: String,
}
impl From<&CairoEvent> for CheckpointEvent {
    fn from(value: &CairoEvent) -> Self {
        Self {
            name: value.to_js_function_name_string(),
            function: value.to_js_function_string(),
        }
    }
}

#[derive(Debug)]
pub struct GeneratedFile {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Error)]
pub(crate) enum GeneratorError {}

pub(crate) fn generate_indexer(
    file_domains: &[FileDomain],
) -> Result<Vec<GeneratedFile>, GeneratorError> {
    let mut files = Vec::new();
    for domain in file_domains {
        if domain.events.is_empty() {
            continue;
        }

        files.push(generate_model(&domain));
        files.push(generate_data_writer(&domain));
    }
    files.push(generate_config(file_domains));

    Ok(files)
}

fn generate_model(domain: &FileDomain) -> GeneratedFile {
    let content = generate_graphql_model(&domain.name, &domain.events);
    GeneratedFile {
        name: String::from(format!("{}.gql", &domain.name)),
        content,
    }
}

fn generate_graphql_model(name: &str, events: &[CairoEvent]) -> String {
    format!(
        r#"
scalar Text

type {} {{
    id: String!
    {}
}}
"#,
        name,
        events_to_graphql(events)
    )
}

fn events_to_graphql(events: &[CairoEvent]) -> String {
    events
        .iter()
        .map(|event| event.to_string())
        .collect::<Vec<String>>()
        .join("\n")
}

fn generate_config(domain: &[FileDomain]) -> GeneratedFile {
    let content: CheckpointConfiguration = domain.into();
    let content_str = serde_json::to_string_pretty(&content).unwrap();
    GeneratedFile {
        name: "configuration.json".to_string(),
        content: content_str,
    }
}

fn generate_data_writer(domain: &FileDomain) -> GeneratedFile {
    let content = generate_data_writer_content(&domain.events);
    GeneratedFile {
        name: String::from(format!("{}DataWriter.js", &domain.name)),
        content,
    }
}

fn generate_data_writer_content(events: &[CairoEvent]) -> String {
    format!(
        r#"
import type {{ CheckpointWriter }} from '@snapshot-labs/checkpoint';

export async function handleDeploy() {{
    new Error('Not implemented yet !');
}}

{}
        "#,
        events_to_js_function(events),
    )
}

fn events_to_js_function(events: &[CairoEvent]) -> String {
    events
        .iter()
        .map(|e| e.to_js_function())
        .collect::<Vec<_>>()
        .join("\n")
}
