use crate::builtin::{Contributor, Value};

pub(super) struct FilesContributor {}

impl FilesContributor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Contributor for FilesContributor {
    fn contribute(&self, value: Value) -> Vec<Value> {
        let s = match value {
            Value::String(s) => s,
            Value::Number(v) => v.to_string(),
        };

        let mut result = Vec::new();

        let (suffix, postfix) = match s.rfind('/') {
            Some(i) => (&s[..i], &s[i + 1..]),
            None => (".", &s[..]),
        };


        let path = std::fs::read_dir(suffix);
        if path.is_err() { return result; }

        let path = path.unwrap();

        for entry in path {
            if entry.is_err() { continue; }
            let entry = entry.unwrap();
            let name = entry.file_name();
            let name = name.to_str();
            if name.is_none() { continue; }

            if postfix.starts_with(postfix) {
                result.push(Value::String(format!("{}/{}", suffix, name.unwrap())));
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::builtin::annotator::tests::annotate_with_default;
    use super::*;

    #[test]
    fn test_file_contributor() {
        let annotations = annotate_with_default("$ cd(\"^\")");

        let files: Vec<String> = std::fs::read_dir(".").unwrap()
            .into_iter()
            .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
            .map(|x| format!("\"{}/{}\"", ".", x))
            .collect();

        assert_eq!(annotations.completions(), &files);
    }

}