use regex::Regex;

pub(crate) fn remove_footnotes(s: String) -> String {
    let re = Regex::new(r"\[\d+\]").unwrap();
    let result = re.replace_all(s.as_str(), "");
    result.into_owned()
}

pub(crate) fn replace_commas_in_quotes(s: &str) -> String {
    let re = Regex::new(r#""[^"]*""#).unwrap();
    let result = re.replace_all(s, |caps: &regex::Captures| {
        let matched = caps.get(0).unwrap().as_str();
        matched.replace(",", "")
    });
    result.into_owned()
}

pub(crate) fn find_rfc_links(s: &str) -> Vec<String> {
    let re = Regex::new(r"\[RFC(\d+)\]").unwrap();
    let mut links = Vec::new();

    for cap in re.captures_iter(s) {
        let rfc_number = &cap[1];
        let link = format!("https://datatracker.ietf.org/doc/html/rfc{}", rfc_number);
        links.push(link);
    }

    links
}
