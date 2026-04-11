pub(crate) fn normalize_outgoing_local_markdown_links(text: &str) -> String {
    text.to_owned()
}

#[cfg(test)]
mod tests {
    use super::normalize_outgoing_local_markdown_links;

    #[test]
    fn preserves_unix_absolute_paths_inside_markdown_links() {
        let input = "[open](/Volumes/Extend/Projects/Writer/_open/test.md)";
        let output = normalize_outgoing_local_markdown_links(input);
        assert_eq!(output, input);
    }

    #[test]
    fn preserves_spaces_and_non_ascii_in_local_file_links() {
        let input = "[report](/Volumes/Extend/Projects/Writer/시장 분석/report final.md)";
        let output = normalize_outgoing_local_markdown_links(input);
        assert_eq!(output, input);
    }

    #[test]
    fn preserves_line_fragments_in_local_file_links() {
        let input =
            "[code](/Volumes/Extend/Projects/DevWorkspace/xsfire-camp/src/codex_agent.rs#L257)";
        let output = normalize_outgoing_local_markdown_links(input);
        assert_eq!(output, input);
    }

    #[test]
    fn preserves_existing_uris_and_plain_text() {
        let input = concat!(
            "[web](https://example.com)\n",
            "[file](file:///Volumes/Extend/Projects/Writer/_open/test.md)\n",
            "plain /Volumes/Extend/Projects/Writer/_open/test.md"
        );
        let output = normalize_outgoing_local_markdown_links(input);
        assert_eq!(output, input);
    }

    #[test]
    fn preserves_windows_absolute_paths_and_angle_wrapped_destinations() {
        let input = "[win](<C:\\Users\\g\\Documents\\report final.md>)";
        let output = normalize_outgoing_local_markdown_links(input);
        assert_eq!(output, input);
    }
}
