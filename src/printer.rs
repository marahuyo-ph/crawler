pub fn pretty_printer(value: serde_json::Value) -> anyhow::Result<String> {
    let mut output = String::new();

    // For root objects with a single key that's an object, unwrap it
    if let serde_json::Value::Object(map) = &value {
        if map.len() == 1 {
            if let Some((title, nested_val)) = map.iter().next() {
                if let serde_json::Value::Object(nested_map) = nested_val {
                    // Print header with the title
                    output.push_str("╭─ ");
                    output.push_str(title);
                    output.push(' ');
                    let line_width = 60;
                    let current_len = output.lines().last().unwrap_or("").len();
                    if current_len < line_width {
                        output.push_str(&"─".repeat(line_width.saturating_sub(current_len)));
                    }
                    output.push('\n');

                    // Print the nested object's entries directly
                    let entries: Vec<_> = nested_map.iter().collect();
                    let len = entries.len();
                    for (idx, (key, val)) in entries.iter().enumerate() {
                        let is_last = idx == len - 1;
                        let prefix = if is_last { "╰─" } else { "├─" };

                        output.push_str(prefix);
                        output.push(' ');
                        output.push_str(key);

                        match val {
                            serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                                output.push_str(": ");
                                output.push('\n');
                                format_value(val, &mut output, 1, false);
                            }
                            _ => {
                                output.push_str(": ");
                                format_value(val, &mut output, 0, false);
                                output.push('\n');
                            }
                        }
                    }

                    // Print closing border
                    output.push_str("╰");
                    output.push_str(&"─".repeat(59));
                    output.push('\n');
                    return Ok(output);
                }
            }
        }
    }

    // Fall back to normal formatting for other structures
    format_value(&value, &mut output, 0, true);
    Ok(output)
}

fn format_value(value: &serde_json::Value, output: &mut String, depth: usize, is_root: bool) {
    match value {
        serde_json::Value::Object(map) => {
            format_object(map, output, depth, is_root);
        }
        serde_json::Value::Array(arr) => {
            format_array(arr, output, depth, is_root);
        }
        serde_json::Value::String(s) => {
            output.push_str(s);
        }
        serde_json::Value::Number(n) => {
            output.push_str(&n.to_string());
        }
        serde_json::Value::Bool(b) => {
            output.push_str(&b.to_string());
        }
        serde_json::Value::Null => {
            output.push_str("null");
        }
    }
}

fn format_object(
    map: &serde_json::Map<String, serde_json::Value>,
    output: &mut String,
    depth: usize,
    _is_root: bool,
) {
    if map.is_empty() {
        return;
    }

    let entries: Vec<_> = map.iter().collect();
    let len = entries.len();

    // Print entries
    for (idx, (key, val)) in entries.iter().enumerate() {
        let is_last = idx == len - 1;
        let prefix = if is_last { "└─" } else { "├─" };

        let indent = "│  ".repeat(depth);

        output.push_str(&indent);
        output.push_str(prefix);
        output.push(' ');
        output.push_str(key);

        match val {
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                output.push_str(": ");
                output.push('\n');
                format_value(val, output, depth + 1, false);
            }
            _ => {
                output.push_str(": ");
                format_value(val, output, depth, false);
                output.push('\n');
            }
        }
    }
}

fn format_array(arr: &[serde_json::Value], output: &mut String, depth: usize, _is_root: bool) {
    if arr.is_empty() {
        return;
    }

    let indent = "│  ".repeat(depth);

    for (idx, val) in arr.iter().enumerate() {
        let is_last = idx == arr.len() - 1;
        let prefix = if is_last { "└─" } else { "├─" };

        output.push_str(&indent);
        output.push_str(prefix);
        output.push(' ');

        match val {
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                output.push('\n');
                format_value(val, output, depth + 1, false);
            }
            _ => {
                format_value(val, output, depth, false);
                output.push('\n');
            }
        }
    }
}
