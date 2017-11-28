use serde_json;
use std::collections::BTreeSet;
use super::{LalResult, Manifest, Lockfile};


/// A single update of of a propagation
#[derive(Serialize)]
pub struct SingleUpdate {
    /// Where to update dependencies
    pub repo: String,
    /// Dependencies to update
    pub dependencies: Vec<String>,
}

/// A parallelizable update stage of a propagation
#[derive(Serialize, Default)]
pub struct UpdateStage {
    /// Updates to perform at this stage
    pub updates: Vec<SingleUpdate>,
}

/// A set of sequential update steps that describe a propagation
#[derive(Serialize, Default)]
pub struct UpdateSequence {
    /// Update stages needed
    pub stages: Vec<UpdateStage>,
}

/// Compute the update sequence for a propagation
pub fn compute(lf: &Lockfile, component: &str) -> LalResult<UpdateSequence> {
    // 1. collect the list of everything we want to build in between root and component
    let all_required = lf.get_reverse_deps_transitively_for(component.into());
    let dependencies = lf.find_all_dependency_names(); // map String -> Set(names)

    debug!("Needs updating: {:?}", all_required);
    debug!("Dependency table: {:?}", dependencies);

    // initialize mutables
    let mut result = UpdateSequence::default();
    let mut remaining = all_required.clone();
    // assume we already updated the component itself
    let mut handled = vec![component.to_string()].into_iter().collect();

    // create update stages while there is something left to update
    while !remaining.is_empty() {
        let mut stage = UpdateStage::default();
        debug!("Remaining set: {:?}", remaining);

        for dep in remaining.clone() {
            debug!("Processing {}", dep);
            // Consider transitive deps for dep, and check they are not in remaining
            let deps_for_name = dependencies[&dep].clone();
            debug!("Deps for {} is {:?}", dep, deps_for_name);
            let intersection = deps_for_name.intersection(&remaining).collect::<BTreeSet<_>>();
            debug!("Intersection: {:?}", intersection);
            if intersection.is_empty() {
                // what to update is `handled` intersected with deps for this repo
                stage.updates.push(SingleUpdate {
                                       repo: dep,
                                       dependencies: deps_for_name
                                           .intersection(&handled)
                                           .cloned()
                                           .collect(),
                                   });
            }
        }

        // remove what we are doing in this stage from remaining
        for dep in &stage.updates {
            remaining.remove(&dep.repo);
            handled.insert(dep.repo.clone());
        }
        result.stages.push(stage);
    }
    Ok(result)
}


/// Outputs the update path to the current manifest for a specific component
///
/// Given a component to propagate to the current one in your working directory,
/// work out how to propagate it through the dependency tree fully.
///
/// This will produce a set of sequential steps, each set itself being parallelizable.
/// The resulting update steps can be performed in order to ensure `lal verify` is happy.
pub fn print(manifest: &Manifest, component: &str, json_output: bool) -> LalResult<()> {
    debug!("Calculating update path for {}", component);

    // TODO: allow taking a custom lockfile to be used outside a repo.
    let lf = Lockfile::default().set_name(&manifest.name).populate_from_input()?;

    let result = compute(&lf, component)?;

    if json_output {
        let encoded = serde_json::to_string_pretty(&result)?;
        println!("{}", encoded);
    } else {
        println!("Assuming {} has been updated:", component);
        let mut i = 1;
        for stage in result.stages {
            println!("Stage {}:", i);
            for update in stage.updates {
                println!("- update [{}] in {}",
                         update.dependencies.join(", "),
                         update.repo);
            }
            i += 1;
        }
    }

    Ok(())
}
