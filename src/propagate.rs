use std::collections::BTreeSet;
use super::{LalResult, Manifest, Lockfile};


/// Outputs the update path to the current manifest
///
/// Given a component to propagate to the current one in your working directory,
/// work out how to propagate it through the dependency tree fully.
///
/// This will produce a set of sequential steps, each set itself being parallelizable.
/// The resulting steps can be executed in order to ensure `lal verify` is happy.
pub fn propagate(manifest: &Manifest, component: &str) -> LalResult<()> {
    debug!("Calculating update path for {}", component);

    // TODO: allow taking a custom lockfile to be used outside a repo.
    let lf = Lockfile::default()
        .set_name(&manifest.name)
        .populate_from_input()?;


    // algorithm
    // collect the list of everything we want to build in between root and component
    let mut remaining = lf.get_reverse_deps_transitively_for(component.into());
    let mut stages : Vec<Vec<String>> = vec![]; // starts out empty
    // TODO: a stage should also contain what to bump inside dep..

    let dependencies = lf.find_all_dependency_names();

    debug!("Needs updating: {:?}", remaining);
    debug!("Dependency table: {:?}", dependencies);

    while !remaining.is_empty() {
        let mut stage = vec![];

        let remainingset = remaining.clone().into_iter().collect::<BTreeSet<_>>();
        debug!("Remaining set: {:?}", remainingset);

        for dep in remaining.clone() {
            debug!("Processing {}", dep);
            // Consider transitive deps for dep, and check they are not in remaining
            let deps_for_name = dependencies[&dep].clone();
            debug!("Deps for {} is {:?}", dep, deps_for_name);
            let intersection = deps_for_name.intersection(&remainingset).collect::<BTreeSet<_>>();
            debug!("Intersection: {:?}", intersection);
            if intersection.is_empty() {
                stage.push(dep);
            }
        }

        // remove what we are doing in this stage from remaining
        for dep in &stage {
            remaining.remove(dep);
        }
        stages.push(stage);
    }
    for stage in stages {
        println!("stage: {:?}", stage);
    }
    Ok(())
}
