use super::{LalResult, Lockfile, Manifest};
use serde_json;
use std::collections::BTreeSet;
use std::collections::HashMap;

/// Repo with channel
#[derive(Clone, Debug, Serialize)]
pub struct RepoWithChannel {
    component: String,
    channel: String,
}

/// Possible dependency formats.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Repo {
    /// Name only
    Short(String),
    /// Name and channel
    Long(RepoWithChannel),
}

impl std::fmt::Display for Repo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Repo::Short(s) => write!(f, "{}", s),
            Repo::Long(c) => write!(f, "{}={}", c.component, c.channel),
        }
    }
}

/// A single update of of a propagation
#[derive(Debug, Serialize)]
pub struct SingleUpdate {
    /// Where to update dependencies
    pub repo: Repo,
    /// Dependencies to update
    pub dependencies: Vec<Repo>,
}

/// A parallelizable update stage of a propagation
#[derive(Debug, Serialize, Default)]
pub struct UpdateStage {
    /// Updates to perform at this stage
    pub updates: Vec<SingleUpdate>,
}

/// A set of sequential update steps that describe a propagation
#[derive(Debug, Serialize, Default)]
pub struct UpdateSequence {
    /// Update stages needed
    pub stages: Vec<UpdateStage>,
}

/// Compute the update sequence for a propagation
pub fn compute(lf: &Lockfile, components: &[String]) -> LalResult<UpdateSequence> {
    // 1. collect the list of everything we want to build in between root and component
    let all_required = components
        .iter()
        .map(|c| lf.get_reverse_deps_transitively_for(c.to_owned()))
        .fold(BTreeSet::<String>::new(), |mut acc, r| {
            for c in r {
                acc.insert(c);
            }
            acc
        });
    let dependencies = lf.find_all_dependency_names(); // map String -> Set(names)
    let channels = lf.find_all_channels(); // map String -> Set(Option(channel))

    debug!("Needs updating: {:?}", all_required);
    debug!("Dependency table: {:?}", dependencies);
    debug!("Channel table: {:?}", channels);

    // initialize mutables
    let mut result = UpdateSequence::default();
    let mut remaining = all_required.clone();
    // assume we already updated the component itself
    let mut handled = components
        .iter()
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();

    // create update stages while there is something left to update
    while !remaining.is_empty() {
        let mut stage = UpdateStage::default();
        debug!("Remaining set: {:?}", remaining);

        for repo in remaining.clone() {
            debug!("Processing {}", repo);
            // Consider transitive deps for dep, and check they are not in remaining
            let deps_for_name = dependencies[&repo].clone();
            debug!("Deps for {} is {:?}", repo, deps_for_name);
            let intersection = deps_for_name
                .intersection(&remaining)
                .collect::<BTreeSet<_>>();
            debug!("Intersection: {:?}", intersection);
            if intersection.is_empty() {
                // what to update is `handled` intersected with deps for this repo
                stage.updates.push(SingleUpdate {
                    repo: dep_to_repo(&repo, &channels),
                    dependencies: deps_for_name
                        .intersection(&handled)
                        .map(|d| dep_to_repo(d, &channels))
                        .collect(),
                });
            }
        }

        // remove what we are doing in this stage from remaining
        for dep in &stage.updates {
            let repo = match &dep.repo {
                Repo::Short(s) => s,
                Repo::Long(r) => &r.component,
            };
            remaining.remove(repo);
            handled.insert(repo.clone());
        }
        result.stages.push(stage);
    }
    Ok(result)
}

fn dep_to_repo(component: &str, channels: &HashMap<String, BTreeSet<Option<String>>>) -> Repo {
    let channels = channels.get(component);
    let channel = match channels {
        Some(channels) => {
            let channels = channels.iter().filter_map(Clone::clone).collect::<Vec<_>>();
            match channels.len() {
                0 => None,
                1 => Some(channels[0].clone()),
                _ => panic!("Multiple channels found for {}: {:?}", component, channels),
            }
        }
        _ => None,
    };

    let component = component.to_owned();
    match channel {
        None => Repo::Short(component),
        Some(channel) => Repo::Long(RepoWithChannel { component, channel }),
    }
}

/// Outputs the update path to the current manifest for a specific component
///
/// Given a component to propagate to the current one in your working directory,
/// work out how to propagate it through the dependency tree fully.
///
/// This will produce a set of sequential steps, each set itself being parallelizable.
/// The resulting update steps can be performed in order to ensure `lal verify` is happy.
pub fn print(manifest: &Manifest, components: &[String], json_output: bool) -> LalResult<()> {
    debug!("Calculating update path for components: {:?}", components);

    // TODO: allow taking a custom lockfile to be used outside a repo.
    let lf = Lockfile::default()
        .set_name(&manifest.name)
        .with_channel(manifest.channel.clone())
        .populate_from_input()?;

    let result = compute(&lf, &components)?;

    if json_output {
        let encoded = serde_json::to_string_pretty(&result)?;
        println!("{}", encoded);
    } else {
        println!(
            "Assuming the following components been updated: {:?}",
            components
        );
        let mut i = 1;
        for stage in result.stages {
            println!("Stage {}:", i);
            for update in stage.updates {
                println!(
                    "- update [{}] in {}",
                    update
                        .dependencies
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<String>>()
                        .join(", "),
                    update.repo
                );
            }
            i += 1;
        }
    }

    Ok(())
}
