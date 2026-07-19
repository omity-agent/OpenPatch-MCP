use super::{Failure, FileStats, OperationOutput};
impl OperationOutput {
    pub(crate) fn structured(&self) -> rmcp::serde_json::Value {
        rmcp :: serde_json :: json ! ({ "succeeded" : self . succeeded () , "successes" : self . successes . iter () . map (| success | { rmcp :: serde_json :: json ! ({ "kind" : success . kind . tag () , "path" : success . path . display () . to_string () , "before" : success . before . map (FileStats :: structured) , "after" : success . after . map (FileStats :: structured) , "uuid" : success . uuid . to_string () , "undoOf" : success . undo_of . map (| uuid | uuid . to_string ()) , }) }) . collect ::< Vec < _ >> () , "failures" : self . failures . iter () . map (Failure :: structured) . collect ::< Vec < _ >> () , })
    }
}
impl FileStats {
    fn structured(self) -> rmcp::serde_json::Value {
        rmcp :: serde_json :: json ! ({ "lineCount" : self . line_count , "characterCount" : self . character_count , })
    }
}
impl Failure {
    fn structured(&self) -> rmcp::serde_json::Value {
        let operation = self . operation . as_ref () . map (| operation | { let kind = operation . 0 ; let path = & operation . 1 ; rmcp :: serde_json :: json ! ({ "kind" : kind . tag () , "path" : path . display () . to_string () , }) }) ;
        rmcp :: serde_json :: json ! ({ "operation" : operation , "undoUuid" : self . undo_uuid , "reason" : self . reason , })
    }
}
