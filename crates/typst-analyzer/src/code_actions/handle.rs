use std::collections::HashMap;

use regex::Regex;
use tower_lsp::lsp_types::*;

use crate::backend::Backend;

pub trait TypstCodeActions {
    fn get_table_parameters(&self) -> HashMap<String, String>;
    fn parse_table_params(&self, content: &str) -> Vec<String>;
    fn calculate_code_actions(
        &self,
        content: &str,
        range: Range,
        uri: Url,
    ) -> Vec<CodeActionOrCommand>;
}

/*    // code action for each params for table function
#table(
    columns: auto,
    rows: auto,
    gutter: auto,
    column-gutter: auto,
    row-gutter: auto,
    fill: none,
    align: auto,
    stroke: none,
    inset: relative,
)
    */
impl TypstCodeActions for Backend {
    fn get_table_parameters(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("columns".to_owned(), "auto".to_owned());
        params.insert("rows".to_owned(), "auto".to_owned());
        params.insert("gutter".to_owned(), "auto".to_owned());
        params.insert("column-gutter".to_owned(), "auto".to_owned());
        params.insert("row-gutter".to_owned(), "auto".to_owned());
        params.insert("fill".to_owned(), "none".to_owned());
        params.insert("align".to_owned(), "auto".to_owned());
        params.insert("stroke".to_owned(), "none".to_owned());
        params.insert("inset".to_owned(), "relative".to_owned());
        params
    }

    /// Parses the content inside `#table(...)` and extracts the parameters already defined.
    fn parse_table_params(&self, content: &str) -> Vec<String> {
        // Regular expression to find parameters (e.g., `param:`).
        let re = Regex::new(r"(\w+(-\w+)?)\s*:").unwrap();
        let mut existing_params = Vec::new();

        for cap in re.captures_iter(content) {
            if let Some(param) = cap.get(1) {
                existing_params.push(param.as_str().to_owned());
            }
        }
        existing_params
    }

    fn calculate_code_actions(
        &self,
        content: &str,
        range: Range,
        uri: Url,
    ) -> Vec<CodeActionOrCommand> {
        let mut actions = Vec::new();

        // Check if the text "VS Code" is within the range
        let vs_code_re = Regex::new(r"VS Code").unwrap();

        for (line_idx, line) in content.lines().enumerate() {
            if let Some(vs_code_match) = vs_code_re.find(line) {
                let start = vs_code_match.start();
                let end = vs_code_match.end();

                // Ensure the match is within the specified range
                if line_idx == range.start.line as usize && line_idx == range.end.line as usize {
                    let edit = TextEdit {
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: start as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: end as u32,
                            },
                        },
                        new_text: "Neovim".to_owned(),
                    };

                    let workspace_edit = WorkspaceEdit {
                        changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
                        document_changes: None,
                        change_annotations: None,
                    };

                    let code_action = CodeAction {
                        title: "Replace 'VS Code' with 'Neovim'".to_owned(),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: None,
                        edit: Some(workspace_edit),
                        command: None,
                        is_preferred: Some(true),
                        disabled: None,
                        data: None,
                    };

                    // Wrap CodeAction in CodeActionOrCommand
                    actions.push(CodeActionOrCommand::CodeAction(code_action));
                }
            }
        }

        let mut multiline_table = String::new();
        let mut in_table_block = false;
        let mut table_start_line = 0;

        for (line_idx, line) in content.lines().enumerate() {
            // Handle multi-line `#table(...)` blocks.
            if line.contains("#table(") {
                in_table_block = true;
                table_start_line = line_idx;
            }

            if in_table_block {
                multiline_table.push_str(line);
                multiline_table.push('\n');

                if line.contains(")") {
                    in_table_block = false;

                    // Extract existing parameters inside `#table(...)`.
                    let existing_params: Vec<String> = self.parse_table_params(&multiline_table);

                    // Get all default parameters.
                    let all_params: HashMap<String, String> = self.get_table_parameters();

                    // Generate a separate code action for each missing parameter.
                    for (param, default_value) in all_params {
                        if !existing_params.contains(&param) {
                            let title = format!("Add missing parameter: {}", param);

                            // Create a new parameter string.
                            let new_param = format!("{}: {},\n  ", param, default_value);
                            // Prepare the text edit to add the missing parameter.
                            let edit = TextEdit {
                                range: Range {
                                    start: Position {
                                        line: table_start_line as u32 + 1,
                                        character: 2,
                                        // line: table_start_line as u32,
                                        // character: line.find("#table(").unwrap_or(5) as u32 + 7, // Position after `#table(`.
                                    },
                                    end: Position {
                                        line: table_start_line as u32 + 1,
                                        character: 2,
                                        // line: table_start_line as u32,
                                        // character: line.find("#table(").unwrap_or(0) as u32 + 7,
                                    },
                                },
                                new_text: new_param,
                            };

                            // Define the workspace edit for the code action.
                            let workspace_edit = WorkspaceEdit {
                                changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
                                document_changes: None,
                                change_annotations: None,
                            };

                            // Create the code action for adding the missing parameter.
                            let code_action = CodeAction {
                                title,
                                kind: Some(CodeActionKind::QUICKFIX),
                                diagnostics: None,
                                edit: Some(workspace_edit),
                                command: None,
                                is_preferred: Some(true),
                                disabled: None,
                                data: None,
                            };

                            // Add the code action to the list.
                            actions.push(CodeActionOrCommand::CodeAction(code_action));
                        }
                    }

                    // Reset the multiline table content for the next block.
                    multiline_table.clear();
                }
            }
        }

        actions
    }
}