use crate::Config;


/// Returns the JS code within `<script>` tags.
pub(crate) fn script(config: &Config) -> String {
    const JS_CODE: &str = include_str!("generated/browser.js");

    JS_CODE.replace("{{ control_path }}", &config.control_path)
}

/// Injects our JS code into `input`. This function tries to find the closing
/// `body` tag and insert the script right before it.
pub(crate) fn into(input: &[u8], config: &Config) -> Vec<u8> {
    // Try to find the closing `body` tag.
    let mut body_close_idx = None;
    let mut inside_comment = false;
    for i in 0..input.len() {
        let rest = &input[i..];
        if !inside_comment && rest.starts_with(b"</body>") {
            body_close_idx = Some(i);
        } else if !inside_comment && rest.starts_with(b"<!--") {
            inside_comment = true;
        } else if inside_comment && rest.starts_with(b"-->") {
            inside_comment = false;
        }
    }

    // If we haven't found a closing body tag, we just insert our JS at the very
    // end.
    let insert_idx = body_close_idx.unwrap_or(input.len());

    let control_path = &config.control_path;
    let script_tag = format!(r#"<script src="{control_path}/client.js" defer></script>"#);

    let mut out = input[..insert_idx].to_vec();
    out.extend_from_slice(script_tag.as_bytes());
    out.extend_from_slice(&input[insert_idx..]);
    out
}
