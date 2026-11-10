use super::{
    super::{
        OperationService,
        result::{Failure, OperationOutput},
    },
    durable, success, workspace,
};
use crate::parser::FileHunk;
use alloc::sync::Arc;
pub(super) fn execute(service: &OperationService, hunks: Vec<FileHunk>) -> OperationOutput {
    let mut output = OperationOutput::default();
    let guard = match service.begin() {
        Ok(guard) => guard,
        Err(error) => {
            let reason = Arc::new(error);
            for hunk in &hunks {
                let (kind, path) = crate::patch::hunk_context(hunk);
                output.push_failure(Failure::file(kind, path, Arc::clone(&reason)));
            }
            return output;
        }
    };
    let plan = workspace::plan(hunks);
    for failure in plan.failures {
        output.push_failure(failure);
    }
    if plan.eliminated_all {
        output.push_failure(Failure::global(String::from(
            "No effective file operations remained after merging.",
        )));
    }
    if !plan.mutations.is_empty() {
        match durable::commit(&service.history, &plan.mutations, None) {
            Ok(ids) => {
                for (planned, id) in plan.mutations.iter().zip(ids) {
                    output.push_success(success(&planned.mutation, id));
                }
            }
            Err(error) => {
                let reason = Arc::new(error);
                for planned in plan.mutations {
                    output.push_failure(Failure::file(
                        planned.mutation.kind,
                        planned.mutation.display_path,
                        Arc::clone(&reason),
                    ));
                }
            }
        }
    }
    drop(guard);
    output
}
