pub fn wiki_escape(s: &str) -> String {
    s.trim()
        .replace('\r', "")
        .replace('\n', "\\\\")
        .replace('{', "")
        .replace('}', "")
}
